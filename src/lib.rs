use std::collections::HashMap;
use serde::Serialize;
use serde::de::Error;
use serde_json::Value;
use web_sys::Document;
use web_sys::console;
use web_sys::window;
use web_sys::Element;
use web_sys::HtmlInputElement;
use web_sys::HtmlTextAreaElement;
use web_sys::HtmlImageElement;
use web_sys::Event;
use web_sys::EventTarget;
use wasm_bindgen::prelude::*;

pub trait TemplateExt {

    fn mount_on(&self, host : Element) -> Template<'_, (), ()>;

    fn mount_on_body(&self) -> Template<'_, (), ()>;

    fn mount_on_id(&self, id : &str) -> Template<'_, (), ()>;
}

impl<A> TemplateExt for A where A : AsRef<str> {

    fn mount_on(&self, host : Element) -> Template<'_, (), ()> {
        Template { html : self.as_ref(), host : Some(host), data : None, callback : None }
    }

    fn mount_on_body(&self) -> Template<'_, (), ()>{

        let body = window().unwrap().document().unwrap().body().unwrap().dyn_into::<Element>().unwrap();
        
        Template { html : self.as_ref(), host : Some(body), data : None, callback : None }
    }

    fn mount_on_id(&self, id : &str) -> Template<'_, (), ()>{

        if let Some(window) = window(){
            if let Some(document) = window.document() {
                if let Ok(option) = document.query_selector(&format!("#{}", id)) {
                    if let Some(element) = option {
                        return Template { html : self.as_ref(), host : Some(element), data : None, callback : None }
                    }else{
                        console::error_1(&format!("Host '{}' not found", id).into());
                    }
                }
            }
        }

        Template { html : self.as_ref(), host : None, data : None, callback : None }
    }
}

pub struct Template<'a, D, F>{
    html : &'a str,
    host : Option<Element>,
    data : Option<D>,
    callback : Option<F>
}

impl Template<'_, (), ()> {
    pub fn from_id(id : &str) -> String {

        if let Some(window) = window(){
            if let Some(document) = window.document() {
                if let Ok(option) = document.query_selector(&format!("#{}", id)) {
                    if let Some(element) = option {
                        return element.inner_html()
                    }else{
                        console::error_1(&format!("Template '{}' not found", id).into());
                    }
                }
            }
        }

        "".to_string()
    }
}

impl<'a, D, F> Template<'a, D, F> where D :  serde::Serialize {

    pub fn clear(self) -> Template<'a, (), ()> {

        if let Some(host) = &self.host {
            while let Some(child) = host.first_child() {
                host.remove_child(&child).unwrap();
            }

            Template {
                html: self.html,
                host: self.host,
                data: None,
                callback: None,
            }
        }else{
            Template {
                html: self.html,
                host: None,
                data: None,
                callback: None,
            }
        }
    }

    pub fn with_data<D2>(self, data : D2) -> Template<'a, D2, F> where D2 : Serialize {

        Template { html : self.html, host: self.host, data: Some(data), callback: self.callback }
    }

    pub fn with_callback<F2>(self, callback : F2) -> Template<'a, D, F2>  where F2 : FnMut(Context) + 'static  {

        Template { html : self.html, host: self.host, data: self.data, callback: Some(callback) }
    }

    fn inner_render<F2>(&self, mut callback : Option<&mut F2>) where F2 : FnMut(Context) {

        if let Some(window) = window() {

            if let Some(document) = window.document() {

                let value = match self.data.as_ref() {
                    Some(data) => serde_json::to_value(data).expect_throw("Data serialization failed"),
                    None => serde_json::Value::Object(serde_json::Map::new()),
                };

                let data_array = match value {
                    Value::Array(arr) => Value::Array(arr),
                    Value::Object(obj) => Value::Array(vec![Value::Object(obj)]),
                    _ => { Value::Array([].to_vec()) }
                };

                if let Value::Array(items) = data_array {
                    
                    for data in items {

                        let mut ui : HashMap<String, Element> = HashMap::new();

                        if let Ok(container) = document.create_element("div") {

                            container.set_inner_html(self.html);

                            if let Some(container) = container.first_element_child(){

                                let mut node_list = container.query_selector_all("*").unwrap();

                                for index in 0..node_list.length() {
                                    let node = node_list.item(index).unwrap();
                                        
                                    let element : Element = node.dyn_into().unwrap();

                                    if let Some(include) = element.get_attribute("pv-include") {
                                        let template = Template::from_id(&include);
                                        element.set_inner_html(&template);
                                    }
                                }
                                
                                node_list = container.query_selector_all("*").unwrap();

                                for index in 0..node_list.length() {
                                    let node = node_list.item(index).unwrap();
                                        
                                    let element : Element = node.dyn_into().unwrap();

                                    if let Some(tag) = element.get_attribute("pv-tag") {
                                        ui.insert(tag, element.clone());
                                    }
                                }

                                for index in 0..node_list.length() {

                                    let node = node_list.item(index).unwrap();

                                    let element : Element = node.dyn_into().unwrap();

                                    self.process_element(element, &mut ui, &data);
                                }

                                if let Some(host) = &self.host {

                                    host.append_child(&container).unwrap();

                                    self.process_element(container, &mut ui, &data);

                                    if let Some(callback) = callback.as_mut() {
                                        callback(Context { ui, data : Some(data), document : document.clone() } );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn process_element(&self, element : Element, ui : &mut HashMap<String, Element>, data : &Value){

        let arrtibutes = element.get_attribute_names();

        for attribute in arrtibutes {

            if let Some(name) = attribute.as_string() {

                if name.starts_with("pv-"){

                    if let Some(value) = element.get_attribute(&name) {

                        let property = Property::new(name.to_string(), value);

                        if let Err(e) = self.execute_attribute(&element, &property, ui, &data){
                            console::error_1(&e);
                        }
                    }
                }
            }
        }
    }

    fn execute_attribute(&self, element : &Element, property : &Property, ui : &mut HashMap<String, Element>, data : &Value) -> Result<(), JsValue> {

        if property.contains(vec!["pv-visible", "pv-show"]){

            let (field, condition) = match property.value.split_once(":"){
                Some((field, condition)) => {
                    
                    if condition.trim().to_lowercase() == "" || condition.trim().to_lowercase() == "true" {
                        (&field.to_string(), true)
                    }else{
                        (&field.to_string(), false)
                    }
                },
                None => { (&property.value, true) }
            };

            let data_value = data.get(field)
                .cloned() 
                .ok_or_else(|| JsValue::from_str(&format!("Data field '{}' not found", field)))?;

            if let Some(field_value) = data_value.as_bool() {

                if field_value != condition {

                    if property.name == "pv-visible" {
                        element.remove();
                    }

                    if property.name == "pv-show" {
                        element.set_attribute("hidden", "true").expect_throw("Cannot set attribute hidden");
                    }
                }
            }
        }

        if property.contains(vec!["pv-toggle", "pv-toggle-class", "pv-switch"]){

            if property.is("pv-toggle"){ 

                let target_element = ui.get(&property.value).expect_throw("Not found");
                    
                let target_element_clone = target_element.clone();

                element.on_click( move |_| {
                    if target_element_clone.has_attribute("hidden") {
                        target_element_clone.remove_attribute("hidden").expect_throw("Cannot remove attribute hidden");
                    }else{
                        target_element_clone.set_attribute("hidden", "true").expect_throw("Cannot set attribute hidden");
                    }
                });
            }

            if property.is("pv-toggle-class"){ 

                if let Some((target_element_name, class_name)) = &property.value.split_once(":"){
                    let target_element = ui.get(&target_element_name.to_string()).expect_throw("Not found");
                        
                    let target_element_clone = target_element.clone();
                    let class_name_clone = class_name.to_string();

                    element.on_click( move |_| {
                        if target_element_clone.class_list().contains(&class_name_clone) {
                            target_element_clone.class_list().remove_1(&class_name_clone).expect_throw("Cannot remove css class");
                        }else{
                            target_element_clone.class_list().add_1(&class_name_clone).expect_throw("Cannot add css class");
                        }
                    });
                }
            }

            if property.is("pv-switch"){ 

                if let Some((on, off)) = property.value.split_once(":") {

                    let mut on_elements : Vec<Element> = Vec::new();
                    let mut off_elements : Vec<Element> = Vec::new();

                    for target_element_name in on.split(","){
                        let target_element = ui.get(target_element_name).expect_throw("Switch target element not found");
                        on_elements.push(target_element.clone());
                    }

                    for target_element_name in off.split(","){
                        let target_element = ui.get(target_element_name).expect_throw("Switch target element not found");
                        off_elements.push(target_element.clone());
                    }

                    element.on_click(move |_| {

                        for element in on_elements.iter() {
                            element.remove_attribute("hidden").expect_throw("Cannot remove attribute hidden");
                        }

                        for element in off_elements.iter() {
                            element.set_attribute("hidden", "true").expect_throw("Cannot set attribute hidden");
                        }
                    });
                }
            }
        }

        if property.contains(vec!["pv-foreach"]){

            let (field, child, condition) = match property.value.split_once(":"){
                Some((field, condition)) => {

                    let (parent_field, child_field) = match field.split_once("."){
                        Some((parent_field, child_field)) => {
                            (parent_field, Some(child_field))
                        },
                        None => {
                            (field, None)
                        }
                    };
                    
                    let condition_value = if condition.trim().to_lowercase() == "" || condition.trim().to_lowercase() == "true" {
                        true
                    }else{
                        false
                    };

                    (parent_field, child_field, condition_value)
                },
                None => { (property.value.as_str(), None, true) }
            };
                
            let data_value = data.get(field)
                .cloned() 
                .ok_or_else(|| JsValue::from_str(&format!("Data field '{}' not found", field)))?;

            let template_name = element.get_attribute("pv-template").unwrap();

            if let Some(array) = data_value.as_array() {

                for item in array {

                    if let Some(field) = child {
                        let value = item.get(field).unwrap();

                        if let Some(bool) = value.as_bool() {
                            if condition == bool {
                                Template::from_id(&template_name)
                                    .mount_on(element.clone())
                                    .with_data(item)
                                    .render();
                            }
                        }
                    }else{
                        Template::from_id(&template_name)
                            .mount_on(element.clone())
                            .with_data(item)
                            .render();
                    }
                }
            }
        }

        if property.contains(vec!["pv-text", "pv-css", "pv-value", "pv-checked", "pv-src"]){

            let data_value = data.get(&property.value)
                .cloned() 
                .ok_or_else(|| JsValue::from_str(&format!("Data field '{}' not found", property.value)))?;

            if property.is("pv-text") {

                if let Some(text) = data_value.as_str(){
                    element.set_text_content(Some(text));
                }
            }

            if property.is("pv-css") {

                if let Some(cls) = data_value.as_str(){
                    element.set_class_name(cls);
                }
            }

            if property.is("pv-value") {

                if let Some(text) = data_value.as_str(){

                    if let Some(input) = element.dyn_ref::<HtmlInputElement>(){
                        input.set_value(text);
                    }
                    else if let Some(input) = element.dyn_ref::<HtmlTextAreaElement>(){
                        input.set_value(text);
                    }
                }
            }

            if property.is("pv-checked") {

                if let Some(bool) = data_value.as_bool(){

                    if let Some(input) = element.dyn_ref::<HtmlInputElement>(){ 
                        match input.type_().as_str() {
                            "radio" | "checkbox" => { 
                                if bool { console::log_1(&JsValue::from(bool));
                                    input.set_checked(true);
                                }
                            },
                            _ => {}
                        }
                    }
                }
            }

            if property.is("pv-src") {
                if let Some(value) = data_value.as_str(){
                    if let Some(img) = element.dyn_ref::<HtmlImageElement>(){
                        img.set_src(value);
                    }  
                }
            }
        }

        element.remove_attribute(&property.name)?;

        Ok(())
    }
}

impl<'a, D> Template<'a, D, ()> where D : serde::Serialize {

    pub fn render(self) {
        self.inner_render::<fn(Context)>(None);
    }
}

impl<'a, D, F> Template<'a, D, F> where D : serde::Serialize, F : FnMut(Context) + 'static {

    pub fn render(&mut self) {
        let mut callback = self.callback.take(); 
        self.inner_render(callback.as_mut());
        self.callback = callback;
    }
}

pub trait ElementExt {

    fn on_event<F>(&self, event_name : &str, callback: F) where F: 'static + FnMut(Event);

    fn on_click<F>(&self, callback: F) where F: 'static + FnMut(Event);
}

impl ElementExt for Element {

    fn on_event<F>(&self, event_name : &str, callback: F) where F: 'static + FnMut(Event){

        let target: &EventTarget = self.dyn_ref().unwrap();

        let closure = Closure::wrap(Box::new(callback) as Box<dyn FnMut(_)>);

        target
            .add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())
            .unwrap();

        closure.forget();
    }

    fn on_click<F>(&self, callback: F) where F: 'static + FnMut(Event){

        self.on_event("click", callback);
    }
}

#[derive(Debug)]
struct Property {
    name : String,
    value : String
}

impl Property {
    
    fn new(name : String, value : String) -> Self {
        Self {
            name, value
        }
    }

    fn is(&self, name : &str) -> bool {
        self.name.eq_ignore_ascii_case(name)
    }
    
    fn contains(&self, properties : Vec<&str>) -> bool{
        properties.contains(&self.name.as_str())
    }
}

pub struct Context{
    ui : HashMap<String, Element>,
    data : Option<serde_json::Value>,
    document : Document
}

impl Context {

    pub fn ui(&self) -> &HashMap<String, Element> {
        &self.ui
    }

    pub fn data<T>(&mut self) -> Result<T, serde_json::error::Error> where
        T: serde::de::DeserializeOwned {

        match self.data.take() {
            Some(value) => serde_json::from_value(value),
            None => Err(serde_json::error::Error::custom("Context data already taken")),
        }
    }

    pub fn document(&self) -> Document{
        self.document.clone()
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
                        &format!("Missing field: {}", stringify!($field)).into()
                    );
                    return;
                }
            };
        )*
    };
}