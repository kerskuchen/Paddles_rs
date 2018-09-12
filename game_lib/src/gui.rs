/// Immediate mode gui that is heavily inspired by the tutorials of
/// Jari Komppa of http://sol.gfxile.net/imgui/index.html
///
use draw;
use draw::{DrawContext, DrawSpace};
use math;
use math::{CanvasPoint, Rect};
use std;
use *;

type ElemId = usize;

#[derive(Debug, Default)]
pub struct GuiContext {
    mouse_pos_canvas: CanvasPoint,
    mouse_is_down: bool,

    keyboard_highlight: Option<ElemId>,
    key_entered: Option<Key>,
    key_modifier: Option<Modifier>,

    last_widget: Option<ElemId>,

    highlighted_item: Option<ElemId>,
    active_item: Option<ElemId>,
}

impl GuiContext {
    pub fn start(&mut self, mouse_pos_canvas: CanvasPoint, input: &GameInput) {
        self.mouse_is_down = input.mouse_button_left.is_pressed;
        self.mouse_pos_canvas = mouse_pos_canvas;
        self.highlighted_item = None;

        self.key_entered = if input.tab_button.is_pressed
            && input.tab_button.num_state_transitions > 0
        {
            Some(Key::Tab)
        } else if input.enter_button.is_pressed && input.enter_button.num_state_transitions > 0 {
            Some(Key::Enter)
        } else if input.left_up_button.is_pressed && input.left_up_button.num_state_transitions > 0
        {
            Some(Key::Up)
        } else if input.left_down_button.is_pressed
            && input.left_down_button.num_state_transitions > 0
        {
            Some(Key::Down)
        } else if input.right_up_button.is_pressed
            && input.right_up_button.num_state_transitions > 0
        {
            Some(Key::Up)
        } else if input.right_down_button.is_pressed
            && input.right_down_button.num_state_transitions > 0
        {
            Some(Key::Down)
        } else if input.left_button.is_pressed && input.left_button.num_state_transitions > 0 {
            Some(Key::Left)
        } else if input.right_button.is_pressed && input.right_button.num_state_transitions > 0 {
            Some(Key::Right)
        } else {
            None
        };

        if input.shift_button.num_state_transitions > 0 {
            self.key_modifier = if input.shift_button.is_pressed {
                Some(Modifier::Shift)
            } else {
                None
            }
        }
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

        if self.key_entered == Some(Key::Tab) {
            self.keyboard_highlight = None;
        }
        self.key_entered = None;
    }

    pub fn button(&mut self, id: ElemId, button_rect: Rect, dc: &mut DrawContext) -> bool {
        if self.mouse_pos_canvas.intersects_rect(button_rect) {
            self.highlighted_item = Some(id);
            if self.active_item.is_none() && self.mouse_is_down {
                self.active_item = Some(id);
            }
        }

        if self.keyboard_highlight.is_none() {
            self.keyboard_highlight = Some(id);
        }

        if self.keyboard_highlight == Some(id) {
            dc.draw_rect_filled(
                button_rect.extended_uniformly_by(2.0),
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
        dc.draw_rect_filled(button_rect, 0.0, color, 0.0, DrawSpace::Canvas);

        if self.keyboard_highlight == Some(id) {
            if let Some(key) = self.key_entered {
                match key {
                    Key::Enter => return true,
                    Key::Tab => {
                        if self.key_modifier == Some(Modifier::Shift) {
                            self.keyboard_highlight = self.last_widget;
                        } else {
                            self.keyboard_highlight = None;
                        }
                    }
                    Key::Up => self.keyboard_highlight = self.last_widget,
                    Key::Down => self.keyboard_highlight = None,
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
                    Key::Tab => {
                        if self.key_modifier == Some(Modifier::Shift) {
                            self.keyboard_highlight = self.last_widget;
                        } else {
                            self.keyboard_highlight = None;
                        }
                    }
                    Key::Up => self.keyboard_highlight = self.last_widget,
                    Key::Down => self.keyboard_highlight = None,
                    Key::Left => {
                        if self.key_modifier == Some(Modifier::Shift) {
                            return Some(math::clamp(cur_value - 0.01, 0.0, 1.0));
                        } else {
                            return Some(math::clamp(cur_value - 0.1, 0.0, 1.0));
                        }
                    }
                    Key::Right => {
                        if self.key_modifier == Some(Modifier::Shift) {
                            return Some(math::clamp(cur_value + 0.01, 0.0, 1.0));
                        } else {
                            return Some(math::clamp(cur_value + 0.1, 0.0, 1.0));
                        }
                    }
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
