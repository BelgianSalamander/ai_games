use crate::langs::python::Python;

use self::language::Language;

pub mod language;
pub mod files;

pub mod python;
pub mod javascript;

pub fn get_all_languages() -> Vec<Box<dyn Language>> {
    vec![
        Box::new(Python)
    ]
}