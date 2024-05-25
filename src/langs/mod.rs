use std::sync::Arc;

use crate::langs::{python::Python, cpp::CppLang};

use self::language::Language;

pub mod language;
pub mod files;

pub mod python;
pub mod javascript;
pub mod cpp;

pub fn get_all_languages() -> Vec<Arc<dyn Language>> {
    vec![
        Arc::new(Python),
        Arc::new(CppLang)
    ]
}