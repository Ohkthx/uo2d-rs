use sdl2::{pixels::Color, rect::Rect, render::WindowCanvas};

use crate::components::{Bounds, Transform, Vec2, Vec3};

pub struct Camera {
    transform: Transform,
}

impl Camera {
    pub fn new(position: Vec3, size: Vec2) -> Self {
        let bounds = Bounds::from_vec(position, size);
        Self {
            transform: Transform::from_bounds(bounds),
        }
    }

    /// Centers the camera on a coordinate.
    pub fn center_on(&mut self, coord: Vec3) {
        let offset = Vec3::from_vec2(
            self.bounding_box().dimensions().apply_scalar(0.5),
            coord.z(),
        );

        self.transform.set_position(&coord.offset_from_2d(&offset));
    }

    /// Checks if a transform is in current view.
    pub fn in_view(&self, other: &Transform) -> bool {
        self.transform
            .bounding_box()
            .intersects_2d(&other.bounding_box())
    }

    pub fn bounding_box(&self) -> Bounds {
        self.transform.bounding_box()
    }

    /// Center of the window.
    pub fn true_center(&self) -> Vec2 {
        Vec2::new(
            self.bounding_box().width() / 2.,
            self.bounding_box().height() / 2.,
        )
    }

    /// Center based on current coordinate.
    #[allow(dead_code)]
    pub fn center(&self) -> Vec2 {
        self.bounding_box().center_2d()
    }

    /// Offset from the center.
    pub fn center_offset(&self, coord: &Vec3) -> Vec2 {
        self.true_center().offset_from(&coord.as_vec2())
    }

    /// Draws a transform to the canvas.
    pub fn draw(&self, canvas: &mut WindowCanvas, object: &Transform, border: u32, color: Vec3) {
        // Prevent drawing items not inview.
        if !self.in_view(object) {
            return;
        }

        // Modify the position based on where the camera is.
        let pos = object.position().offset_from_2d(&self.transform.position());
        let size = object.bounding_box().dimensions();

        if border != 0 {
            // Draw the border
            let border_rect = Rect::new(
                pos.x().round() as i32,
                pos.y().round() as i32,
                object.bounding_box().width().round() as u32,
                object.bounding_box().height().round() as u32,
            );

            canvas.set_draw_color(Color::RGB(0, 0, 0));
            if let Err(why) = canvas.fill_rect(border_rect) {
                eprintln!("Unable to render border: {}", why);
            }
        }

        // Draw the base square on top of the border
        let rect = Rect::new(
            pos.x().round() as i32 + (border as i32),
            pos.y().round() as i32 + (border as i32),
            size.x().round() as u32 - (border * 2),
            size.y().round() as u32 - (border * 2),
        );

        // Convert the color and draw the rect..
        let rgb = color.as_vec().map(|c| c.round().clamp(0., 255.) as u8);
        canvas.set_draw_color(Color::RGB(rgb[0], rgb[1], rgb[2]));
        if let Err(why) = canvas.fill_rect(rect) {
            eprintln!("Unable to render base: {}", why);
        }
    }
}
