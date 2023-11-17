use async_trait::async_trait;
use deadpool::unmanaged::Pool;
use gamedef::game_interface::{GameInterface, self, EnumVariants, BuiltinType, Type, get_enum_variant_type, is_basic_enum};

use crate::isolate::sandbox::{RunningJob, IsolateSandbox, LaunchOptions, DirMapping};

use super::{language::{Language, PreparedProgram}, files::ClientFiles};

pub struct Python;

pub fn type_as_inline_python(ty: &Type) -> String {
    match ty {
        Type::Builtin(ty) => match ty {
            BuiltinType::U8 | BuiltinType::U16 | BuiltinType::U32 | BuiltinType::U64 => {
                "int".to_string()
            }
            BuiltinType::I8 | BuiltinType::I16 | BuiltinType::I32 | BuiltinType::I64 => {
                "int".to_string()
            }
            BuiltinType::F32 | BuiltinType::F64 => "float".to_string(),
            BuiltinType::Bool => "bool".to_string(),
            BuiltinType::Str => "str".to_string(),
        },
        Type::NamedType(name) => name.clone(),
        Type::Array(ty, len) => format!("List[{}]", type_as_inline_python(ty)),
        Type::DynamicArray(ty) => format!("List[{}]", type_as_inline_python(ty)),

        _ => panic!("Cannot convert type {:?} to python inline type", ty),
    }
}

fn write_inline_decoder(ty: &Type) -> String {
    match ty {
        Type::Builtin(ty) => match ty {
            BuiltinType::U8 => "read_u8()",
            BuiltinType::U16 => "read_u16()",
            BuiltinType::U32 => "read_u32()",
            BuiltinType::U64 => "read_u64()",

            BuiltinType::I8 => "read_i8()",
            BuiltinType::I16 => "read_i16()",
            BuiltinType::I32 => "read_i32()",
            BuiltinType::I64 => "read_i64()",

            BuiltinType::F32 => "read_f32()",
            BuiltinType::F64 => "read_f64()",

            BuiltinType::Bool => "read_bool()",
            BuiltinType::Str => "read_str()",
        }
        .to_string(),
        Type::NamedType(name) => format!("read_{}()", name),
        Type::Array(ty, len) => {
            let inner = write_inline_decoder(ty);

            format!("[{} for _ in range({})]", inner, len)
        },
        Type::DynamicArray(ty) => {
            let inner = write_inline_decoder(ty);

            format!("[{} for _ in range(read_u32())]", inner)
        }

        _ => panic!("Cannot convert type {:?} to python inline type", ty),
    }
}

fn write_encoder(ty: &Type, value: &str, indent: u32) -> String {
    let indent_str = "    ".repeat(indent as usize);

    match ty {
        Type::Builtin(ty) => match ty {
            BuiltinType::U8 => format!("{}write_u8({})", indent_str, value),
            BuiltinType::U16 => format!("{}write_u16({})", indent_str, value),
            BuiltinType::U32 => format!("{}write_u32({})", indent_str, value),
            BuiltinType::U64 => format!("{}write_u64({})", indent_str, value),

            BuiltinType::I8 => format!("{}write_i8({})", indent_str, value),
            BuiltinType::I16 => format!("{}write_i16({})", indent_str, value),
            BuiltinType::I32 => format!("{}write_i32({})", indent_str, value),
            BuiltinType::I64 => format!("{}write_i64({})", indent_str, value),

            BuiltinType::F32 => format!("{}write_f32({})", indent_str, value),
            BuiltinType::F64 => format!("{}write_f64({})", indent_str, value),

            BuiltinType::Bool => format!("{}write_bool({})", indent_str, value),
            BuiltinType::Str => format!("{}write_str({})", indent_str, value),
        },
        Type::NamedType(name) => format!("{}write_{}({})", indent_str, name, value),
        Type::Array(ty, len) => {
            let inner = write_encoder(ty, "x", indent + 1);

            format!("{}for x in {}:\n{}", indent_str, value, inner)
        }
        Type::DynamicArray(ty) => {
            let inner = write_encoder(ty, "x", indent + 1);

            format!(
                "{}write_u32(len({}))\n{}for x in {}:\n{}",
                indent_str, value, indent_str, value, inner
            )
        }

        _ => panic!("Cannot convert type {:?} to python inline type", ty),
    }
}

#[async_trait]
impl Language for Python {
    fn name(&self) -> &'static str {
        "Python 3"
    }

    fn id(&self) -> &'static str {
        "python3"
    }

    fn extension(&self) -> &'static str {
        "py"
    }

    fn generate(
        &self,
        game_interface: &GameInterface,
    ) -> ClientFiles {
        let game_interface = game_interface.reduced();

        let mut res = ClientFiles::new();

        res.include_client_file("interact_lib.py", "run");

        let mut type_defs = String::new();

        type_defs.push_str("from dataclasses import dataclass\n");
        type_defs.push_str("from enum import Enum\n");
        type_defs.push_str("from typing import List, ClassVar\n\n");

        let mut types = vec![];
        for (name, ty) in &game_interface.types {
            match ty {
                game_interface::Type::Struct(fields) => {
                    type_defs.push_str(&format!("@dataclass\n"));
                    type_defs.push_str(&format!("class {}:\n", name));

                    for field in fields.iter() {
                        type_defs.push_str(&format!(
                            "    {}: {}\n",
                            field.name,
                            type_as_inline_python(&field.ty)
                        ));
                    }

                    type_defs.push_str("\n\n");

                    types.push(name.clone());
                }
                game_interface::Type::Enum(variants) => {
                    if is_basic_enum(&variants) {
                        type_defs.push_str(&format!("class {}(Enum):\n", name));

                        for (i, variant) in variants.iter().enumerate() {
                            type_defs.push_str(&format!("    {} = {}\n", variant.name, i));
                        }
                    } else {
                        type_defs.push_str(&format!("class {}:\n", name));

                        for (idx, variant) in variants.iter().enumerate() {
                            type_defs.push_str("    @dataclass\n");
                            type_defs.push_str(&format!("    class {}:\n", variant.name));

                            type_defs.push_str(&format!(
                                "        VARIANT_ID: ClassVar[int] = {}\n",
                                idx
                            ));

                            for field in variant.types.iter() {
                                type_defs.push_str(&format!(
                                    "        {}: {}\n",
                                    field.name,
                                    type_as_inline_python(&field.ty)
                                ));
                            }

                            type_defs.push_str("\n");
                        }

                        type_defs.push_str("    def __init__(self, data):\n");
                        type_defs.push_str("        self.data = data\n");
                    }

                    type_defs.push_str("\n\n");

                    types.push(name.clone());
                }
                _ => {}
            }
        }

        res.add_file("run/game_types.py", type_defs, false);

        let mut template = String::new();

        template.push_str("from game_types import ");

        for (i, ty) in types.iter().enumerate() {
            if i != 0 {
                template.push_str(", ");
            }

            template.push_str(ty);
        }

        template.push_str("\n");
        template.push_str("from typing import List\n\n");

        for (name, signature) in &game_interface.functions {
            template.push_str(&format!("def {}(", name));

            for (i, (name, ty)) in signature.args.iter().enumerate() {
                if i != 0 {
                    template.push_str(", ");
                }

                template.push_str(&format!("{}: {}", name, type_as_inline_python(ty)));
            }

            if let Some(ret) = &signature.ret {
                template.push_str(&format!(") -> {}", type_as_inline_python(ret)));
            } else {
                template.push_str(")");
            }

            template.push_str(":\n");
            template.push_str("    pass\n\n");
        }

        res.add_file("game.py", template, false);

        let mut interactor = String::new();

        interactor.push_str("from interact_lib import *\n");
        interactor.push_str("from game import *\n\n");

        for (name, ty) in &game_interface.types {
            match ty {
                Type::Struct(fields) => {
                    interactor.push_str(&format!("def read_{}():\n    return {}(\n", name, name));

                    for field in fields.iter() {
                        interactor.push_str(&format!(
                            "        {}={},\n",
                            field.name,
                            write_inline_decoder(&field.ty)
                        ));
                    }

                    interactor.push_str("    )\n\n");

                    interactor.push_str(&format!("def write_{}(value):\n", name));

                    for field in fields.iter() {
                        interactor.push_str(&format!(
                            "{}\n",
                            write_encoder(&field.ty, &format!("value.{}", field.name), 1)
                        ));
                    }

                    interactor.push_str("\n\n");
                }
                Type::Enum(variants) => {
                    if is_basic_enum(variants) {
                        interactor.push_str(&format!(
                            "def read_{}():\n    return {}({})\n\n",
                            name, name,
                            write_inline_decoder(&Type::Builtin(get_enum_variant_type(
                                variants
                            )))
                        ));
                        interactor.push_str(&format!(
                            "def write_{}(value):\n    {}\n\n\n",
                            name,
                            write_encoder(
                                &Type::Builtin(get_enum_variant_type(variants)),
                                "value.value",
                                0
                            )
                        ));
                    } else {
                        for variant in variants.iter() {
                            interactor.push_str(&format!(
                                "def read_enum_variant_{}_{}():\n    return {}.{}(\n",
                                name, variant.name, name, variant.name
                            ));

                            for field in variant.types.iter() {
                                interactor.push_str(&format!(
                                    "        {}={},\n",
                                    field.name,
                                    write_inline_decoder(&field.ty)
                                ));
                            }

                            interactor.push_str("    )\n\n");
                        }

                        interactor.push_str(&format!("ENUM_VARIANT_READERS_{} = [\n", name));

                        for variant in variants.iter() {
                            interactor.push_str(&format!(
                                "    read_enum_variant_{}_{},\n",
                                name, variant.name
                            ));
                        }

                        interactor.push_str("]\n\n");

                        interactor.push_str(&format!("def read_{}():\n", name));

                        interactor.push_str(&format!(
                            "    variant_id = {}\n",
                            write_inline_decoder(&Type::Builtin(get_enum_variant_type(
                                variants
                            )))
                        ));

                        interactor.push_str(&format!(
                            "    return {}(ENUM_VARIANT_READERS_{}[variant_id]())\n\n",
                            name, name
                        ));

                        for variant in variants.iter() {
                            interactor.push_str(&format!(
                                "def write_enum_variant_{}_{}(value):\n",
                                name, variant.name
                            ));

                            for field in variant.types.iter() {
                                interactor.push_str(&format!(
                                    "{}\n",
                                    write_encoder(
                                        &field.ty,
                                        &format!("value.data.{}", field.name),
                                        1
                                    )
                                ));
                            }

                            if variant.types.len() == 0 {
                                interactor.push_str("    pass\n");
                            }

                            interactor.push_str("\n");
                        }

                        interactor.push_str(&format!("ENUM_VARIANT_WRITERS_{} = [\n", name));

                        for variant in variants.iter() {
                            interactor.push_str(&format!(
                                "    write_enum_variant_{}_{},\n",
                                name, variant.name
                            ));
                        }

                        interactor.push_str("]\n\n");

                        interactor.push_str(&format!("def write_{}(value):\n", name));

                        interactor.push_str(&write_encoder(
                            &Type::Builtin(get_enum_variant_type(variants)),
                            "value.data.VARIANT_ID",
                            1,
                        ));

                        interactor.push_str(&format!(
                            "\n    ENUM_VARIANT_WRITERS_{}[value.data.VARIANT_ID](value)\n",
                            name
                        ));

                        interactor.push_str("\n\n");
                    }
                }
                ty => {
                    interactor.push_str(&format!(
                        "def read_{}():\n    return {}\n\n",
                        name,
                        write_inline_decoder(ty)
                    ));
                    interactor.push_str(&format!(
                        "def write_{}(value):\n{}\n\n\n",
                        name,
                        write_encoder(ty, "value", 1)
                    ));
                }
            }
        }

        interactor.push_str("def mainloop():\n    while True:\n        func_id = read_u8()");

        for (i, (name, signature)) in game_interface.functions.iter().enumerate() {
            interactor.push_str(&format!("\n        if func_id == {}:\n", i));

            let mut func_call = String::new();

            func_call.push_str(&format!("{}(", name));

            for (i, (name, ty)) in signature.args.iter().enumerate() {
                if i != 0 {
                    func_call.push_str(", ");
                }

                func_call.push_str(&write_inline_decoder(ty));
            }

            func_call.push_str(")");

            if let Some(ret) = &signature.ret {
                interactor.push_str(&format!("            ret = {}\n", func_call));
                interactor.push_str(&format!("{}\n", write_encoder(ret, "ret", 3)));
                interactor.push_str("            flush()\n");
            } else {
                interactor.push_str(&format!("            {}\n", func_call));
            }

            interactor.push_str("            continue\n");
        }

        interactor.push_str("\n\nif __name__ == '__main__':\n    mainloop()");

        res.add_file("run/interactor.py", interactor, false);

        res
    }

    async fn prepare(&self, src: &str, out: &mut PreparedProgram, game_interface: &GameInterface, _sandboxes: Pool<IsolateSandbox>) -> Result<(), String> {
        out.add_src_file("game.py", src);

        Ok(())
    }

    fn launch(&self, data_dir: &str, sandbox: &crate::isolate::sandbox::IsolateSandbox, game_interface: &GameInterface) -> RunningJob {
        sandbox.launch(
            "/usr/bin/python3".to_string(),
            vec!["/prog/run/interactor.py".to_string()], 
            &LaunchOptions::new()
                .map_dir("/prog", self.get_dir(game_interface))
                .map_dir("/game", data_dir)
                .set_env("PYTHONPATH", "/game")
        )
    }
}

unsafe impl Send for Python {}
unsafe impl Sync for Python {}