//! mquickjs-ui: UI 渲染层
//!
//! 基于 slint 的 UI 渲染引擎，为 mquickjs-rs 提供 UI 能力。

use std::collections::HashMap;

/// UI 元素 ID
pub type ElementId = u64;

/// UI 渲染器
pub struct UIRenderer {
    elements: HashMap<ElementId, Element>,
    next_id: ElementId,
}

/// UI 元素
struct Element {
    element_type: String,
    properties: HashMap<String, String>,
    children: Vec<ElementId>,
    parent: Option<ElementId>,
}

impl UIRenderer {
    /// 创建新的 UI 渲染器
    pub fn new() -> Self {
        Self {
            elements: HashMap::new(),
            next_id: 1,
        }
    }

    /// 创建 UI 元素
    pub fn create_element(&mut self, element_type: &str) -> ElementId {
        let id = self.next_id;
        self.next_id += 1;

        let element = Element {
            element_type: element_type.to_string(),
            properties: HashMap::new(),
            children: Vec::new(),
            parent: None,
        };

        self.elements.insert(id, element);
        id
    }

    /// 设置属性
    pub fn set_property(&mut self, element_id: ElementId, key: &str, value: &str) {
        if let Some(element) = self.elements.get_mut(&element_id) {
            element.properties.insert(key.to_string(), value.to_string());
        }
    }

    /// 添加子元素
    pub fn append_child(&mut self, parent_id: ElementId, child_id: ElementId) {
        if let Some(parent) = self.elements.get_mut(&parent_id) {
            parent.children.push(child_id);
        }
        if let Some(child) = self.elements.get_mut(&child_id) {
            child.parent = Some(parent_id);
        }
    }

    /// 获取元素属性
    pub fn get_property(&self, element_id: ElementId, key: &str) -> Option<&str> {
        self.elements
            .get(&element_id)?
            .properties
            .get(key)
            .map(|s| s.as_str())
    }

    /// 获取子元素
    pub fn get_children(&self, element_id: ElementId) -> Vec<ElementId> {
        self.elements.get(&element_id).map(|e| e.children.clone()).unwrap_or_default()
    }

    /// 渲染 UI（简单实现，实际应该调用 slint）
    pub fn render(&self) {
        // TODO: 实现实际的渲染逻辑
        println!("Rendering UI...");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_element() {
        let mut renderer = UIRenderer::new();
        let id = renderer.create_element("div");
        assert!(id > 0);
    }

    #[test]
    fn test_set_property() {
        let mut renderer = UIRenderer::new();
        let id = renderer.create_element("div");
        renderer.set_property(id, "class", "container");
        assert_eq!(renderer.get_property(id, "class"), Some("container"));
    }

    #[test]
    fn test_append_child() {
        let mut renderer = UIRenderer::new();
        let parent = renderer.create_element("div");
        let child = renderer.create_element("span");
        renderer.append_child(parent, child);
        
        let children = renderer.get_children(parent);
        assert_eq!(children.len(), 1);
        assert_eq!(children[0], child);
    }
}