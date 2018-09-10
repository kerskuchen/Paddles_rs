/// Immediate mode gui that is heavily inspired by the tutorials of
/// Jari Komppa of http://sol.gfxile.net/imgui/index.html
///
use draw;
use draw::{DrawContext, DrawSpace};
use math;
use math::{CanvasPoint, Rect};
use std;

type ElemId = usize;

#[derive(Debug, Default)]
pub struct GuiContext {
    next_menu_id: ElemId,
    mouse_pos_canvas: CanvasPoint,
    mouse_is_down: bool,

    highlighted_item: Option<ElemId>,
    active_item: Option<ElemId>,
}

impl GuiContext {
    pub fn start(&mut self, mouse_pos_canvas: CanvasPoint, mouse_is_down: bool) {
        self.mouse_is_down = mouse_is_down;
        self.mouse_pos_canvas = mouse_pos_canvas;
        self.highlighted_item = None;
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
    }

    pub fn button(&mut self, id: ElemId, button_rect: Rect, dc: &mut DrawContext) -> bool {
        if self.mouse_pos_canvas.intersects_rect(button_rect) {
            self.highlighted_item = Some(id);
            if self.active_item.is_none() && self.mouse_is_down {
                self.active_item = Some(id);
            }
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
        max_value: f32,
        dc: &mut DrawContext,
    ) -> Option<f32> {
        let knob_size = slider_rect.dim().y;
        let x_pos = ((slider_rect.dim().x - knob_size) * cur_value) / max_value;
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

        if self.active_item == Some(id) {
            let mouse_x = math::clamp(
                self.mouse_pos_canvas.x - (slider_rect.pos().x),
                0.0,
                slider_rect.dim().x,
            );

            let value = (mouse_x * max_value) / slider_rect.dim().x;
            if value != cur_value {
                return Some(value);
            }
        }
        None
    }
}
