use crate::api::ShapeServiceSingleton;

pub struct DefaultShapeService;

impl ShapeServiceSingleton for DefaultShapeService {
    fn create_circle(&mut self, radius: i32) -> i32 {
        radius * radius
    }

    fn create_rectangle(&mut self, width: i32, height: i32) -> i32 {
        width * height
    }

    fn describe_shape(&mut self, kind: String) -> String {
        format!("Shape: {}", kind)
    }
}

pub fn create_shape_service_singleton() -> Box<dyn ShapeServiceSingleton> {
    Box::new(DefaultShapeService)
}