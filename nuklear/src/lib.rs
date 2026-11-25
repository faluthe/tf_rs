//! Safe wrapper around the [Nuklear](https://github.com/Immediate-Mode-UI/Nuklear) bindings
//! in `nuklear_sys`, with chained row builders:
//!
//! ```ignore
//! nk.row_dynamic(30.0, 2)
//!     .label("FOV", TextAlignment::LEFT)
//!     .slider_int(1, &mut config.aimbot.fov, 100, 1);
//! ```
//!
//! Draws in its own GL context, made current for the frame and then handed back, so the
//! game's GL state isn't touched. Events go to Nuklear before the game, and the ones it
//! uses while the menu is open are cleared so the game ignores them.

mod context;
pub mod flags;
mod icon;
pub mod input;
pub mod nuklear;
pub mod rect;
pub mod style;

pub use icon::Icon;
pub use input::Input;
pub use input::NkKey;
pub use nuklear::{Nuklear, Scroll, Swatch, ToggleColor};
pub use nuklear_sys::SDL_Scancode;
pub use rect::Rect;
pub use style::{Color, Font, Palette};
