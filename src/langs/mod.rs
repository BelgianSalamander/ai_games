use crate::langs::{python::Python, cpp::CppLang};

use self::language::Language;

pub mod language;
pub mod files;

pub mod python;
pub mod javascript;
pub mod cpp;

pub fn get_all_languages() -> Vec<Box<dyn Language>> {
    vec![
        Box::new(Python),
        Box::new(CppLang)
    ]
}