use std::collections::HashMap;
use web_sys::window;
use web_sys::Element;
use web_sys::HtmlInputElement;
use wasm_bindgen::prelude::*;

pub trait TemplateExt {

    fn mount_on(&self, host : Element) -> Template<'_, (), ()>;

    fn mount_on_body(&self) -> Template<'_, (), ()>;
}

impl<A> TemplateExt for A where A : AsRef<str> {

    fn mount_on(&self, host : Element) -> Template<'_, (), ()> {
        
        Template { html : self.as_ref(), host, data : None, callback : None }
    }

    fn mount_on_body(&self) -> Template<'_, (), ()>{

        let body = window().unwrap().document().unwrap().body().unwrap().dyn_into::<Element>().unwrap();
        
        Template { html : self.as_ref(), host : body, data : None, callback : None }
    }
}

pub struct Template<'a, D, F>{
    html : &'a str,
    host : Element,
    data : Option<D>,
    callback : Option<F>
}

impl<'a, D, F> Template<'a, D, F> where D :  serde::Serialize {

    pub fn clear(self) -> Template<'a, (), ()> {

        while let Some(child) = self.host.first_child() {
            self.host.remove_child(&child).unwrap();
        }

        Template {
            html: self.html,
            host: self.host,
            data: None,
            callback: None,
        }
    }

    pub fn with_data<D2>(self, data : D2) -> Template<'a, D2, F>{

        Template { html : self.html, host: self.host, data: Some(data), callback: self.callback }
    }

    pub fn with_callback<F2>(self, callback : F2) -> Template<'a, D, F2> {

        Template { html : self.html, host: self.host, data: self.data, callback: Some(callback) }
    }

    fn inner_render<F2>(self, callback : Option<F2>) where F2 : FnMut(HashMap<String, Element>) {

        if let Some(window) = window() {

            if let Some(document) = window.document() {

                let bind_map = match self.data.as_ref() {
                    Some(data) => serde_json::to_value(data).expect("Serialization failed"),
                    None => serde_json::Value::Object(serde_json::Map::new()),
                };

                let mut ui : HashMap<String, Element> = HashMap::new();

                let container = document.create_element("div").unwrap();
                container.set_inner_html(self.html);

                let node_list = container.query_selector_all("*").unwrap();

                for index in 0..node_list.length() {
                    let node = node_list.item(index).unwrap();

                    let element : Element = node.dyn_into().unwrap();

                    if let Some(name) = element.get_attribute("pv-html") {
                        
                        if let Some(v) = bind_map.get(&name){
                            element.set_inner_html(v.as_str().unwrap());
                        }

                        element.remove_attribute("pv-html").unwrap();
                    }

                    if let Some(name) = element.get_attribute("pv-value") {
                        
                        if let Some(v) = bind_map.get(&name){
                            let input = element.dyn_ref::<HtmlInputElement>().unwrap();
                            input.set_value(v.as_str().unwrap());
                        }

                        element.remove_attribute("pv-value").unwrap();
                    }

                    if let Some(name) = element.get_attribute("pv-visible") {
                        
                        if let Some(value) = bind_map.get(&name){

                            if let Some(boolean) = value.as_bool() {
                                if boolean == false {
                                    element.remove();
                                }
                            }
                            element.remove_attribute("pv-visible").unwrap();
                        }
                    }

                    if let Some(tag) = element.get_attribute("pv-tag") {
                        element.remove_attribute("pv-tag").unwrap();
                        ui.insert(tag, element);
                    }
                }

                if let Some(mut callback) = callback {
                    callback(ui);
                }
                
                self.host.append_child(&container).unwrap();
            }
        }
    }
}

impl<'a, D> Template<'a, D, ()> where D : serde::Serialize {

    pub fn render(self) {

        self.inner_render::<fn(HashMap<String, Element>)>(None);
    }
}

impl<'a, D, F> Template<'a, D, F> where D : serde::Serialize, F : FnMut(HashMap<String, Element>){

    pub fn render(mut self) {
        let callback = self.callback.take();

        self.inner_render(callback);
    }
}

#[macro_export]
macro_rules! tx {
    ($($html:tt)*) => {
        stringify!($($html)*)
    };
}

#[macro_export]
macro_rules! ui {
    ($map:expr, $($field:ident),* $(,)?) => {
        $(
            let $field = match $map.get(stringify!($field)) {
                Some(el) => el.clone(),
                None => {
                    web_sys::console::error_1(
                        &format!("Missing field in map: {}", stringify!($field)).into()
                    );
                    return;
                }
            };
        )*
    };
}
