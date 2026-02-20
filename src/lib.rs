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
                    Some(data) => serde_json::to_value(data).expect("Serialization failed"),
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

                                let node_list = container.query_selector_all("*").unwrap();

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

                if name.starts_with("pv-") && name != "pv-if" && name!="pv-tag" {

                    if let Some(value) = element.get_attribute(&name) {

                        let property = Property::new(name.to_string(), value);

                        if let Err(e) = self.execute_attribute(&element, &property, &data){
                            console::error_1(&e);
                        }
                    }
                }
            }
        }

        if let Some(statement) = element.get_attribute("pv-if") {

            match Parser::compile(statement){

                Ok(conditional_block) => {

                    if let Some(value) = data.get(&conditional_block.identifier){

                        if let Some(boolean) = value.as_bool() {

                            if boolean == conditional_block.condition {

                                for property in conditional_block.block_true {
                                    //self.execute_property(&element, &property, &data);
                                }
                            }else { 
                                if let Some(block_false) = conditional_block.block_false {
                                    for property in block_false {
                                        //self.execute_property(&element, &property, &data);
                                    }
                                }
                            }
                        }
                    }
                },
                Err(e) => {
                    console::error_1(&JsValue::from(&e));
                }   
            }

            element.remove_attribute("pv-if").unwrap();
        }

        if let Some(tag) = element.get_attribute("pv-tag") {
            element.remove_attribute("pv-tag").unwrap();
            ui.insert(tag, element);
        }
    }

    fn execute_attribute(&self, element : &Element, property : &Property, data : &Value) -> Result<(), JsValue> {

        if property.is("pv-visible") {

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
                .ok_or_else(|| JsValue::from_str(&format!("Key '{}' not found", field)))?;

            if let Some(field_value) = data_value.as_bool() {

                if field_value != condition {
                    element.remove();
                }
            }
        }else{

            let data_value = data.get(&property.value)
                .cloned() 
                .ok_or_else(|| JsValue::from_str(&format!("Key '{}' not found", property.value)))?;

            if property.is("pv-text") {

                if let Some(text) = data_value.as_str(){
                    element.set_text_content(Some(text));
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
        }
        
        element.remove_attribute(&property.name)?;

        Ok(())
    }

    fn execute_property(&self, element : &Element, property : &Property, data : &Value) -> Result<(), JsValue> {

        if property.is("visible"){
            if property.value.trim().to_lowercase() == "false" {
                element.remove();
            }
        }

        if property.is("css"){
            element.set_class_name(&property.value);
        }

        if property.is("text"){
            element.set_text_content(Some(&property.value));
        }

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

pub trait Handler {
    fn on_click<F>(&self, callback: F)
    where
        F: 'static + FnMut(Event);
}

impl Handler for Element {
    fn on_click<F>(&self, callback: F)
    where
        F: 'static + FnMut(Event),
    {
        // Cast Element -> EventTarget
        let target: &EventTarget = self.dyn_ref().unwrap();

        // Wrap callback in Closure
        let closure = Closure::wrap(Box::new(callback) as Box<dyn FnMut(_)>);

        // Add the event listener
        target
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();

        // Forget closure to keep it alive for the lifetime of the page
        closure.forget();
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

struct Parser;

impl Parser {

    fn compile(s : String) -> Result<ConditionalBlock, String> {

        let delimiters = vec![' ', ':', ';', '{', '}', '|'];

        let mut token = String::new();

        let mut tokens : Vec<String>  = Vec::new();

        for ch in s.chars() {
            if delimiters.contains(&ch){

                if token.len() > 0 {
                    tokens.push(token.clone());
                }
                
                tokens.push(ch.to_string());
                token.clear();
                continue;
            }

            token.push(ch);
        }

        let token_stream = TokenStream::new(tokens);

        Compile::new(token_stream).compile()
    }
}

#[derive(Debug)]
struct TokenStream {
    tokens : Vec<String>,
    position : usize
}

impl TokenStream {

    fn new(tokens : Vec<String>) -> Self {
        Self {
            tokens,
            position: 0
        }
    }

    fn peek(&self) -> Option<&String> {

        self.tokens.get(self.position)
    }

    fn next(&mut self) -> Option<&String> {

        let token = self.tokens.get(self.position);

        if token.is_some(){
            self.position += 1;
        }

        token
    }

    fn skip_whitespaces(&mut self) {

        while let Some(token) = self.peek() {
            if token.trim().is_empty() {
                self.position += 1;
            }else{
                break;
            }
        }
    }
}
struct Compile {
    token_stream : TokenStream
}

impl Compile {

    fn new(token_stream : TokenStream) -> Self {
        Self  {
            token_stream
        }
    }

    fn compile(&mut self) -> Result<ConditionalBlock, String> {

        let mut conditional_block = ConditionalBlock::default();

        let identifier = self.parse_identifier()?;

        self.expect_token(":")?;

        let condition = self.parse_condition()?;

        let block_true = self.parse_block()?;

        self.token_stream.skip_whitespaces();

        let block_false =  {
            if let Some(_) = self.token_stream.peek(){

                self.expect_token("|")?;

                if let Ok(v) = self.parse_block(){
                    Some(v)
                }else{
                    None
                }
            }else{
                None
            }
        };

        conditional_block.identifier = identifier;
        conditional_block.condition = condition;
        conditional_block.block_true = block_true;
        conditional_block.block_false = block_false;

        Ok(conditional_block)
    }

    fn parse_identifier(&mut self) -> Result<String, String>{

        self.token_stream.skip_whitespaces();

        let field = self.token_stream.next().ok_or("Error".to_string())?;

        Ok(field.to_string())
    }

    fn parse_condition(&mut self) -> Result<bool, String>{

        self.token_stream.skip_whitespaces();

        let condition = self.token_stream.next().ok_or("Error".to_string())?;

        if condition.to_lowercase() == "true"{
            return Ok(true);
        }else if condition.to_lowercase() == "false" {
            return Ok(false);
        }else{
            Err("not a bool".to_string())
        }
    }

    fn parse_block(&mut self) -> Result<Vec<Property>, String> {

        self.token_stream.skip_whitespaces();

        self.expect_token("{")?;

        let mut properties : Vec<Property> = Vec::new();

        self.parse_property(&mut properties)?;

        self.expect_token("}")?;

        Ok(properties)
    }

    fn parse_property(&mut self, properties : &mut Vec<Property>) -> Result<(), String>{

        let property = self.parse_identifier()?;

        self.expect_token(":")?;

        let value = self.parse_value()?;

        self.token_stream.skip_whitespaces();

        properties.push(Property::new(property.trim().to_string(), value.trim().to_string()));

        if let Some(peek) = self.token_stream.peek(){
            if peek == ";" {
                self.token_stream.next().unwrap();
                self.parse_property(properties)?;
            }
        }

        Ok(())
    }

    fn parse_value(&mut self) -> Result<String, String>{

        self.token_stream.skip_whitespaces();

        let field = self.token_stream.next().ok_or("Error".to_string())?;

        Ok(field.to_string())
    }

    fn expect_token(&mut self, identifier : &str) -> Result<(), String>{

        self.token_stream.skip_whitespaces();

        if let Some(ident) = self.token_stream.next() {
            if ident == identifier {
                return Ok(());
            }else{
                return Err(format!("Expected ident {identifier} found {ident}").to_string());
            }
        }

        Err(format!("Expected ident {identifier}").to_string())
    }
}

#[derive(Default, Debug)]
struct ConditionalBlock {
    identifier: String,
    condition: bool,
    block_true: Vec<Property>,
    block_false: Option<Vec<Property>>,
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