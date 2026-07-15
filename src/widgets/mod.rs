//! Concrete widget primitives.
//!
//! Each type in this module implements the [`crate::widget::Widget`] trait
//! and can be spawned into a [`crate::core::UiArena`].

mod button;
mod container;
mod label;

pub use button::Button;
pub use container::Container;
pub use label::Label;
