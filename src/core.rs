use std::collections::VecDeque;

use crate::widget::Widget;

// ---------------------------------------------------------------------------
// WidgetId
// ---------------------------------------------------------------------------

/// Lightweight, copyable handle that refers to a widget inside an [`UiArena`].
///
/// `WidgetId` is just an index into the arena's flat storage.  It carries no
/// lifetime and can be freely copied, stored, and compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidgetId(pub(crate) usize);

// ---------------------------------------------------------------------------
// UiArena
// ---------------------------------------------------------------------------

/// Central, flat storage for the widget tree.
///
/// Widgets live inside a `Vec<Option<Box<dyn Widget<M>>>>`.  Each live slot
/// is `Some(Box<…>)`; `None` marks a vacant entry (available for future
/// reuse if we add removal later).  The generic `M` is the application
/// message type threaded through the entire Elm-style event pipeline.
pub struct UiArena<M> {
    nodes: Vec<Option<Box<dyn Widget<M>>>>,
    root: Option<WidgetId>,
}

impl<M> UiArena<M> {
    /// Creates an empty arena.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            root: None,
        }
    }

    /// Allocates a widget into the arena and returns its unique id.
    pub fn spawn<W: Widget<M> + 'static>(&mut self, widget: W) -> WidgetId {
        let id = WidgetId(self.nodes.len());
        self.nodes.push(Some(Box::new(widget)));
        id
    }

    /// Returns a read-only reference to the widget, if the id is still valid.
    pub fn get(&self, id: WidgetId) -> Option<&(dyn Widget<M> + '_)> {
        let slot = self.nodes.get(id.0)?.as_ref()?;
        Some(&**slot)
    }

    /// Returns a mutable reference to the widget, if the id is still valid.
    pub fn get_mut(&mut self, id: WidgetId) -> Option<&mut (dyn Widget<M> + '_)> {
        let slot = self.nodes.get_mut(id.0)?.as_mut()?;
        Some(&mut **slot)
    }

    /// Designates the given widget as the root of the tree.
    pub fn set_root(&mut self, id: WidgetId) {
        debug_assert!(
            self.get(id).is_some(),
            "set_root: id {id:?} does not exist in the arena"
        );
        self.root = Some(id);
    }

    /// Returns the current root widget, if one has been set.
    pub fn root(&self) -> Option<WidgetId> {
        self.root
    }

    /// Depth-first, top-down traversal starting at `start`.
    ///
    /// The visitor `f` is called with each widget's id and a trait-object
    /// reference **before** its children are visited (pre-order).
    pub fn traverse<F>(&self, start: WidgetId, mut f: F)
    where
        F: FnMut(WidgetId, &dyn Widget<M>),
    {
        let mut stack = vec![start];

        while let Some(id) = stack.pop() {
            if let Some(widget) = self.get(id) {
                f(id, widget);
                // Push children in reverse so that the first child is visited
                // first (stack is LIFO).
                let mut children = widget.children();
                children.reverse();
                for child_id in children {
                    stack.push(child_id);
                }
            }
        }
    }

    /// Breadth-first, top-down traversal starting at `start`.
    ///
    /// Useful when level-order processing is more natural than DFS.
    pub fn traverse_bfs<F>(&self, start: WidgetId, mut f: F)
    where
        F: FnMut(WidgetId, &dyn Widget<M>),
    {
        let mut queue = VecDeque::new();
        queue.push_back(start);

        while let Some(id) = queue.pop_front() {
            if let Some(widget) = self.get(id) {
                f(id, widget);
                for child_id in widget.children() {
                    queue.push_back(child_id);
                }
            }
        }
    }

    /// Returns the total number of live widgets in the arena.
    pub fn len(&self) -> usize {
        self.nodes.iter().filter(|s| s.is_some()).count()
    }

    /// Returns `true` if the arena contains no live widgets.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<M> Default for UiArena<M> {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Mock widgets for testing ------------------------------------------

    struct Label {
        label: &'static str,
        children: Vec<WidgetId>,
    }

    impl Label {
        fn new(label: &'static str) -> Self {
            Self {
                label,
                children: Vec::new(),
            }
        }
    }

    impl Widget<String> for Label {
        fn name(&self) -> &'static str {
            self.label
        }

        fn children(&self) -> Vec<WidgetId> {
            self.children.clone()
        }

        fn add_child(&mut self, child: WidgetId) {
            self.children.push(child);
        }

        fn draw(&self, _canvas: &mut tiny_skia::PixmapMut, _rect: crate::layout::Rect) {}
    }

    struct Container {
        label: &'static str,
        children: Vec<WidgetId>,
    }

    impl Container {
        fn new(label: &'static str) -> Self {
            Self {
                label,
                children: Vec::new(),
            }
        }
    }

    impl Widget<String> for Container {
        fn name(&self) -> &'static str {
            self.label
        }

        fn children(&self) -> Vec<WidgetId> {
            self.children.clone()
        }

        fn add_child(&mut self, child: WidgetId) {
            self.children.push(child);
        }

        fn draw(&self, _canvas: &mut tiny_skia::PixmapMut, _rect: crate::layout::Rect) {}
    }

    // -- Tests -------------------------------------------------------------

    #[test]
    fn spawn_and_get() {
        let mut arena = UiArena::<String>::new();
        let id = arena.spawn(Label::new("hello"));

        let widget = arena.get(id).expect("widget should exist");
        assert_eq!(widget.name(), "hello");
    }

    #[test]
    fn get_mut_and_modify() {
        let mut arena = UiArena::<String>::new();
        let child = arena.spawn(Label::new("child"));
        let parent = arena.spawn(Container::new("Parent"));

        // Mutate through the trait object: add_child is a mut method.
        arena.get_mut(parent).unwrap().add_child(child);
        let children = arena.get(parent).unwrap().children();
        assert_eq!(children, vec![child]);
    }

    #[test]
    fn root_tracking() {
        let mut arena = UiArena::<String>::new();
        assert_eq!(arena.root(), None);

        let id = arena.spawn(Container::new("Root"));
        arena.set_root(id);
        assert_eq!(arena.root(), Some(id));
    }

    #[test]
    fn traverse_dfs_pre_order() {
        // Build:
        //   Root (Container)
        //   ├── A (Label)
        //   │   └── A1 (Label)
        //   └── B (Label)
        let mut arena = UiArena::<String>::new();

        let a1 = arena.spawn(Label::new("A1"));
        let a = arena.spawn(Label::new("A"));
        let b = arena.spawn(Label::new("B"));
        let root = arena.spawn(Container::new("Root"));

        arena.get_mut(a).unwrap().add_child(a1);
        arena.get_mut(root).unwrap().add_child(a);
        arena.get_mut(root).unwrap().add_child(b);

        arena.set_root(root);

        let mut visited = Vec::new();
        arena.traverse(root, |id, widget| {
            visited.push((id, widget.name().to_string()));
        });

        assert_eq!(visited.len(), 4);
        assert_eq!(visited[0].1, "Root");
        // Children are pushed in reverse onto the stack, so A is popped first.
        assert_eq!(visited[1].1, "A");
        assert_eq!(visited[2].1, "A1");
        assert_eq!(visited[3].1, "B");
    }

    #[test]
    fn traverse_bfs_level_order() {
        // Same tree as above, but BFS should visit level by level.
        let mut arena = UiArena::<String>::new();

        let a1 = arena.spawn(Label::new("A1"));
        let a = arena.spawn(Label::new("A"));
        let b = arena.spawn(Label::new("B"));
        let root = arena.spawn(Container::new("Root"));

        arena.get_mut(a).unwrap().add_child(a1);
        arena.get_mut(root).unwrap().add_child(a);
        arena.get_mut(root).unwrap().add_child(b);

        let mut visited = Vec::new();
        arena.traverse_bfs(root, |_, widget| {
            visited.push(widget.name().to_string());
        });

        assert_eq!(visited, vec!["Root", "A", "B", "A1"]);
    }

    #[test]
    fn len_and_is_empty() {
        let mut arena = UiArena::<String>::new();
        assert!(arena.is_empty());

        arena.spawn(Label::new("x"));
        assert_eq!(arena.len(), 1);
        assert!(!arena.is_empty());
    }
}
