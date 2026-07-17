//! Concrete widget primitives.
//!
//! Each type in this module implements the [`crate::widget::Widget`] trait
//! and can be spawned into a [`crate::core::UiArena`].

mod button;
mod column;
mod container;
mod label;
mod row;

pub use button::Button;
pub use column::Column;
pub use container::Container;
pub use label::Label;
pub use row::Row;
