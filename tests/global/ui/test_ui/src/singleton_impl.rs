use crate::api::UISingleton;
use mquickjs_ui::UIRenderer;

pub struct DefaultUI {
    renderer: UIRenderer,
}

impl DefaultUI {
    pub fn new() -> Self {
        Self {
            renderer: UIRenderer::new(),
        }
    }
}

impl UISingleton for DefaultUI {
    fn create_element(&mut self, element_type: String) -> i32 {
        self.renderer.create_element(&element_type) as i32
    }

    fn set_property(&mut self, element_id: i32, key: String, value: String) {
        self.renderer.set_property(element_id as u64, &key, &value);
    }

    fn append_child(&mut self, parent_id: i32, child_id: i32) {
        self.renderer.append_child(parent_id as u64, child_id as u64);
    }

    fn get_property(&mut self, element_id: i32, key: String) -> String {
        self.renderer
            .get_property(element_id as u64, &key)
            .unwrap_or("")
            .to_string()
    }

    fn render(&mut self) {
        self.renderer.render();
    }
}

pub fn create_ui_singleton() -> Box<dyn UISingleton> {
    Box::new(DefaultUI::new())
}