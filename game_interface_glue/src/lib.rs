//! This crates is build as dynamic lib and its whole purpose is to statically link to the
//! [`game_lib`] crate and forward calls to [`game_lib`]s main interface functions.
//!
//! # Problem
//!
//! The reason we need this crate is to make hot loading of the [`game_lib`] crate possible on
//! MS Windows.
//!
//! Let's assume we would just build [`game_lib`] as a dynamic library and try to load it from
//! our [`game_runtime`] crate directly. As we need to know and use datastructures like
//! [`game_lib::DrawCommand`] in our game-runtime, we need to set [`game_lib`] as a dependency
//! of our [`game_runtime`] in its Cargo.toml.
//!
//! This causes our runtime to be effectively linked against [`game_lib`] and triggers the automatic
//! loading of the [`game_lib`] .dll at startup. But this also locks the .dll on MS Windows which
//! in turn prevents recompilations (and thus hot-reloading) of [`game_lib`].
//!
//! # Workaround using this crate
//!
//! To circumvent the problem we use the following constellation:
//! * The runtime depends statically on [`game_lib`]
//! * This crate depends statically on [`game_lib`]
//! * The [`game_runtime`] loads this crates' dynamic lib at runtime
//!
//! Now if we modify [`game_lib`] and want to hotreload it, we just recompile it and this crates
//! dynamic lib (which depends on [`game_lib`]) and reload the dynamic lib from the runtime.
//! Note that the .dll loading mechanism loads a copy of the .dll, so that the original .dll
//! is not locked.
//!
//! # Problems of the workaround
//!
//! The main drawback of this method is that the runtime now has two copies of the [`game_lib`]
//! in its process memory. The one linked statically and the one loaded at runtime. It therefore
//! is important to never call functions of [`game_lib`] directly in the runtime, or the
//! hotreloading won't work.
//!
//! As hot-reloading is mainly used for developing/debugging, it is ok to keep two copies of
//! [`game_lib`] in the runtimes process memory. When publishing the game, the runtime can link to
//! the static [`game_lib`] use its functions directly and just never load this crates dynamic lib.
//!
//! [`game_runtime`]: ../game_runtime/index.html
//!
extern crate game_lib;
use game_lib::{GameContext, GameInput};

/// Forwards directly to [`game_lib::update_and_draw`]
#[no_mangle]
pub fn update_and_draw<'game_context>(
    input: &GameInput,
    game_context: &'game_context mut GameContext<'game_context>,
) {
    game_lib::update_and_draw(input, game_context);
}

/// Forwards directly to [`game_lib::process_audio`]
#[no_mangle]
pub fn process_audio<'game_context>(
    input: &GameInput,
    game_context: &'game_context mut GameContext<'game_context>,
    audio_output_buffer: &mut Vec<f32>,
) {
    game_lib::process_audio(input, game_context, audio_output_buffer);
}
