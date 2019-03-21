/// Immediate mode gui that is heavily inspired by the tutorials of
/// Jari Komppa of http://sol.gfxile.net/imgui/index.html
///
use crate::draw;
use crate::draw::{DrawContext, DrawSpace};
use crate::math;
use crate::math::{CanvasPoint, Rect};
use std;
use crate::utility::CountdownTimer;
use crate::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GuiAction {
    Next,
    Previous,
    Accept,
    Increase,
    Decrease,
}

type ElemId = usize;

#[derive(Debug, Default)]
pub struct GuiContext {
    mouse_pos_canvas: CanvasPoint,
    mouse_is_down: bool,

    keyboard_highlight: Option<ElemId>,
    key_entered: Option<GuiAction>,

    last_widget: Option<ElemId>,

    highlighted_item: Option<ElemId>,
    active_item: Option<ElemId>,
}

impl GuiContext {
    pub fn start(&mut self, mouse_pos_canvas: CanvasPoint, input: &GameInput) {
        self.mouse_is_down = input.mouse_button_left.is_pressed;
        self.mouse_pos_canvas = mouse_pos_canvas;
        self.highlighted_item = None;

        self.key_entered = if input.had_press_event("ui_next") {
            Some(GuiAction::Next)
        } else if input.had_press_event("ui_previous") {
            Some(GuiAction::Previous)
        } else if input.had_press_event("ui_accept") {
            Some(GuiAction::Accept)
        } else {
            None
        };
    }

    pub fn finish(&mut self) {
        if self.mouse_is_down {
            // From http://sol.gfxile.net/imgui/ch03.html
            // "If the mouse is clicked, but no widget is active, we need to mark the active item
            // unavailable so that we won't activate the next widget we drag the cursor onto."
            if self.active_item.is_none() {
                self.active_item = Some(std::usize::MAX);
            }
        } else {
            self.active_item = None;
        }

        if self.key_entered == Some(GuiAction::Next) {
            self.keyboard_highlight = None;
        }
        self.key_entered = None;
    }

    pub fn button(
        &mut self,
        id: ElemId,
        label: &str,
        button_rect: Rect,
        depth: f32,
        dc: &mut DrawContext,
    ) -> bool {
        if self.mouse_pos_canvas.intersects_rect(button_rect) {
            self.highlighted_item = Some(id);
            if self.active_item.is_none() && self.mouse_is_down {
                self.active_item = Some(id);
            }
        }

        if self.keyboard_highlight.is_none() {
            self.keyboard_highlight = Some(id);
        }

        let color = if self.highlighted_item == Some(id) || self.keyboard_highlight == Some(id) {
            if self.active_item == Some(id) {
                draw::COLOR_RED
            } else {
                draw::COLOR_MAGENTA
            }
        } else {
            draw::COLOR_BLUE
        };

        // Draw buttons with outlines
        dc.draw_rect_filled(
            button_rect,
            depth,
            color,
            ADDITIVITY_NONE,
            DrawSpace::Canvas,
        );
        dc.draw_rect(
            button_rect,
            depth,
            Color::new(0.4, 0.4, 0.4, 0.4),
            ADDITIVITY_NONE,
            DrawSpace::Canvas,
        );

        // Draw button text
        let text_rect =
            Rect::from_dimension(dc.get_text_dimensions(label)).centered_in_rect(button_rect);
        dc.draw_text(
            text_rect.pos(),
            label,
            depth,
            COLOR_WHITE,
            ADDITIVITY_NONE,
            DrawSpace::Canvas,
        );

        // Keyboard input
        if self.keyboard_highlight == Some(id) {
            if let Some(key) = self.key_entered {
                match key {
                    GuiAction::Accept => return true,
                    GuiAction::Previous => self.keyboard_highlight = self.last_widget,
                    GuiAction::Next => self.keyboard_highlight = None,
                    _ => {}
                }
                self.key_entered = None;
            }
        }
        self.last_widget = Some(id);

        let button_pressed = self.highlighted_item == Some(id)
            && self.active_item == Some(id)
            && !self.mouse_is_down;

        button_pressed
    }

    pub fn horizontal_slider(
        &mut self,
        id: ElemId,
        slider_rect: Rect,
        cur_value: f32,
        dc: &mut DrawContext,
    ) -> Option<f32> {
        let knob_size = slider_rect.dim().y;
        let x_pos = (slider_rect.dim().x - knob_size) * cur_value;
        let knob_rect = Rect::from_xy_width_height(
            slider_rect.pos().x + x_pos,
            slider_rect.pos().y,
            knob_size,
            knob_size,
        );

        if self.mouse_pos_canvas.intersects_rect(slider_rect) {
            self.highlighted_item = Some(id);
            if self.active_item.is_none() && self.mouse_is_down {
                self.active_item = Some(id);
            }
        }

        // If no widget has keyboard focus, take it
        if self.keyboard_highlight.is_none() {
            self.keyboard_highlight = Some(id);
        }

        if self.keyboard_highlight == Some(id) {
            dc.draw_rect_filled(
                slider_rect.extended_uniformly_by(2.0),
                0.0,
                draw::COLOR_CYAN,
                0.0,
                DrawSpace::Canvas,
            );
        }

        let color = if self.highlighted_item == Some(id) {
            if self.active_item == Some(id) {
                draw::COLOR_RED
            } else {
                draw::COLOR_MAGENTA
            }
        } else {
            draw::COLOR_BLUE
        };
        dc.draw_rect_filled(slider_rect, 0.0, draw::COLOR_WHITE, 0.0, DrawSpace::Canvas);
        dc.draw_rect_filled(knob_rect, 0.0, color, 0.0, DrawSpace::Canvas);

        if self.keyboard_highlight == Some(id) {
            if let Some(key) = self.key_entered {
                match key {
                    GuiAction::Previous => self.keyboard_highlight = self.last_widget,
                    GuiAction::Next => self.keyboard_highlight = None,
                    GuiAction::Decrease => return Some(math::clamp(cur_value - 0.1, 0.0, 1.0)),
                    GuiAction::Increase => return Some(math::clamp(cur_value + 0.1, 0.0, 1.0)),
                    _ => {}
                }
                self.key_entered = None;
            }
        }
        self.last_widget = Some(id);

        if self.active_item == Some(id) {
            let mouse_x = math::clamp(
                self.mouse_pos_canvas.x - (slider_rect.pos().x),
                0.0,
                slider_rect.dim().x,
            );

            let value = mouse_x / slider_rect.dim().x;
            if value != cur_value {
                return Some(value);
            }
        }
        None
    }
}

//==================================================================================================
// FadeOutIn
//==================================================================================================
//

#[derive(Debug, PartialEq)]
enum FadeState {
    FadingOut,
    FadingIn,
    FadedOut,
    FadedIn,
}

#[derive(Debug)]
pub struct ScreenFader {
    fade_timer: CountdownTimer,
    fade_state: FadeState,
}

impl Default for ScreenFader {
    fn default() -> Self {
        ScreenFader {
            fade_timer: CountdownTimer::default(),
            fade_state: FadeState::FadedIn,
        }
    }
}

impl ScreenFader {
    pub fn new() -> ScreenFader {
        ScreenFader::default()
    }

    pub fn fading_overlay_opacity(&self) -> f32 {
        match self.fade_state {
            FadeState::FadedIn => 0.0,
            FadeState::FadedOut => 1.0,
            FadeState::FadingIn => 1.0 - self.fade_timer.completion_ratio(),
            FadeState::FadingOut => self.fade_timer.completion_ratio(),
        }
    }

    pub fn increment(&mut self, delta_time: f32) {
        self.fade_timer.increment(delta_time);
        if self.fade_timer.is_finished() {
            self.fade_state = match self.fade_state {
                FadeState::FadedIn => FadeState::FadedIn,
                FadeState::FadedOut => FadeState::FadedOut,
                FadeState::FadingIn => FadeState::FadedIn,
                FadeState::FadingOut => FadeState::FadedOut,
            };
        }
    }

    pub fn start_fading_out(&mut self, fade_time: f32) {
        self.fade_timer = CountdownTimer::with_given_end_time(fade_time);
        self.fade_state = FadeState::FadingOut;
    }

    pub fn start_fading_in(&mut self, fade_time: f32) {
        self.fade_timer = CountdownTimer::with_given_end_time(fade_time);
        self.fade_state = FadeState::FadingIn;
    }

    pub fn is_fading(&self) -> bool {
        self.fade_state == FadeState::FadingIn || self.fade_state == FadeState::FadingOut
    }

    pub fn has_finished_fading_out(&self) -> bool {
        // NOTE: It only makes sense to call this function if we acually are (or were) fading out
        // debug_assert!(
        //     self.fade_state == FadeState::FadedOut || self.fade_state == FadeState::FadingOut
        // );
        self.fade_state == FadeState::FadedOut
    }

    pub fn has_finished_fading_in(&self) -> bool {
        // NOTE: It only makes sense to call this function if we acually are (or were) fading in
        // debug_assert!(
        //     self.fade_state == FadeState::FadedIn || self.fade_state == FadeState::FadingIn
        // );
        self.fade_state == FadeState::FadedIn
    }
}
