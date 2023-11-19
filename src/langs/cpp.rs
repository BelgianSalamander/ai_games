use async_trait::async_trait;
use deadpool::unmanaged::Pool;
use gamedef::game_interface::{
    get_enum_variant_type, is_basic_enum, BuiltinType, EnumVariants, StructFields, Type,
};

use crate::{
    isolate::sandbox::{
        DirMapping, IsolateSandbox, LaunchOptions, MaxProcessCount, make_public,
    },
    util::temp_file::random_dir,
};

use super::{files::ClientFiles, language::Language};

pub struct CppLang;

pub fn struct_fields(fields: &StructFields, pretty: bool, indent: &str) -> String {
    let mut res = String::new();

    res.push_str("{");
    if pretty {
        res.push_str("\n");
    }

    for field in fields {
        if pretty {
            res.push_str(indent);
            res.push_str("    ");
        }

        res.push_str(&type_as_inline_cpp(&field.ty));

        res.push_str(" ");

        res.push_str(&field.name);

        res.push_str(";");

        if pretty {
            res.push_str("\n");
        }
    }

    if pretty {
        res.push_str(indent);
    }

    res.push_str("}");

    res
}

pub fn full_enum_decl(
    variants: &EnumVariants,
    pretty: bool,
    omit_static: bool,
    name: Option<String>,
) -> String {
    let mut res = String::new();
    if let Some(name) = name {
        res.push_str(&format!("struct {} {{", name))
    } else {
        res.push_str("struct {")
    }

    if pretty {
        res.push_str("\n");
    }

    let variant_type = type_as_inline_cpp(&Type::Builtin(get_enum_variant_type(&variants)));

    if pretty {
        res.push_str("    ");
    }
    res.push_str(&format!("{} variant_id;", variant_type));

    if pretty {
        res.push_str("\n\n    ");
    }

    res.push_str("union EnumMembers {");

    for (idx, variant) in variants.iter().enumerate() {
        if pretty {
            res.push_str("\n        ")
        }
        res.push_str(&format!("struct {} {{", variant.name));

        if pretty {
            res.push_str("\n            ");
        }

        if !omit_static {
            res.push_str(&format!(
                "static const {} VARIANT_ID = {};",
                variant_type, idx
            ));
        }

        for field in &variant.types {
            if pretty {
                res.push_str("\n            ");
            }

            res.push_str(&format!(
                "{} {};",
                type_as_inline_cpp(&field.ty),
                field.name
            ));
        }

        if pretty {
            res.push_str("\n        ");
        }
        res.push_str("}");

        res.push_str(&format!(" {}_val;", variant.name));

        if pretty {
            res.push_str("\n");
        }
    }

    res.push_str("    } inner;");

    if pretty {
        res.push_str("\n");
    }

    res.push_str("}");

    res
}

pub fn basic_enum_decl(variants: &EnumVariants, pretty: bool, name: Option<String>) -> String {
    let mut res = String::new();
    if let Some(name) = name {
        res.push_str(&format!("enum class {} ", name))
    } else {
        res.push_str("enum class ")
    }

    let variant_type = type_as_inline_cpp(&Type::Builtin(get_enum_variant_type(&variants)));

    res.push_str(&format!(": {} {{", variant_type));

    for variant in variants.iter() {
        if pretty {
            res.push_str("\n    ")
        }
        res.push_str(&format!("{},", variant.name));
    }

    if pretty {
        res.push_str("\n");
    }

    res.push_str("}");

    res
}

pub fn enum_decl(variants: &EnumVariants, pretty: bool, name: Option<String>) -> String {
    if is_basic_enum(variants) {
        basic_enum_decl(variants, pretty, name)
    } else {
        full_enum_decl(variants, pretty, false, name)
    }
}

pub fn type_as_inline_cpp(ty: &Type) -> String {
    match ty {
        Type::Builtin(builtin) => (match builtin {
            gamedef::game_interface::BuiltinType::U8 => "uint8_t",
            gamedef::game_interface::BuiltinType::U16 => "uint16_t",
            gamedef::game_interface::BuiltinType::U32 => "uint32_t",
            gamedef::game_interface::BuiltinType::U64 => "uint64_t",
            gamedef::game_interface::BuiltinType::I8 => "int8_t",
            gamedef::game_interface::BuiltinType::I16 => "int16_t",
            gamedef::game_interface::BuiltinType::I32 => "int32_t",
            gamedef::game_interface::BuiltinType::I64 => "int64_t",
            gamedef::game_interface::BuiltinType::F32 => "float",
            gamedef::game_interface::BuiltinType::F64 => "double",
            gamedef::game_interface::BuiltinType::Bool => "bool",
            gamedef::game_interface::BuiltinType::Str => "std::string",
        })
        .to_string(),
        Type::Struct(fields) => format!("struct {}", struct_fields(&fields, false, "")),
        Type::Array(elem, size) => format!("std::array<{},{}>", type_as_inline_cpp(&elem), size),
        Type::DynamicArray(elem) => format!("std::vector<{}>", type_as_inline_cpp(&elem)),
        Type::Enum(variants) => enum_decl(&variants, false, None),
        Type::NamedType(name) => name.clone(),
    }
}

fn write_struct_decoder(
    fields: &StructFields,
    indent: usize,
    base_addr: String,
    out: &mut String,
    discriminant: &mut usize,
) {
    for field in fields.iter() {
        let field_name = &field.name;
        write_decoder(
            &field.ty,
            indent,
            format!("{base_addr}.{field_name}"),
            out,
            discriminant,
        );
    }
}

pub fn write_decoder(
    ty: &Type,
    indent: usize,
    base_addr: String,
    out: &mut String,
    discriminant: &mut usize,
) {
    let indent_str = "    ".repeat(indent);
    match ty {
        Type::Builtin(BuiltinType::Str) => {
            out.push_str(&indent_str);
            out.push_str(&format!("readString({base_addr});\n"));
        }
        Type::Builtin(_) => {
            let name = type_as_inline_cpp(ty);
            out.push_str(&indent_str);
            out.push_str(&format!("readData<{name}>({base_addr});\n"));
        }
        Type::Array(elem, size) => {
            let elem_name = type_as_inline_cpp(elem);

            let new_base_addr = format!("baseAddr_{discriminant}");
            *discriminant += 1;

            out.push_str(&format!(
                "{indent_str}for (int i = 0; i < {size}; i++) {{\n"
            ));
            out.push_str(&format!(
                "{indent_str}    {elem_name}& {new_base_addr} = {base_addr}[i];\n"
            ));
            write_decoder(&elem, indent + 1, new_base_addr, out, discriminant);
            out.push_str(&format!("{indent_str}}}\n"));
        }
        Type::DynamicArray(elem) => {
            let elem_name = type_as_inline_cpp(elem);

            let new_base_addr = format!("baseAddr_{discriminant}");
            let new_size = format!("size_{discriminant}");
            *discriminant += 1;

            out.push_str(&format!(
                "{indent_str}uint32_t {new_size};\n{indent_str}readData<uint32_t>({new_size});\n"
            ));
            out.push_str(&format!(
                "{indent_str}for (int i = 0; i < {new_size}; i++) {{\n"
            ));
            out.push_str(&format!(
                "{indent_str}    {elem_name}& {new_base_addr} = {base_addr}[i];\n"
            ));
            write_decoder(&elem, indent + 1, new_base_addr, out, discriminant);
            out.push_str(&format!("{indent_str}}}\n"));
        }
        Type::NamedType(name) => {
            out.push_str(&format!("{indent_str}read_{name}({base_addr});\n"));
        }
        Type::Struct(fields) => {
            write_struct_decoder(fields, indent, base_addr, out, discriminant);
        }
        Type::Enum(variants) => {
            if is_basic_enum(&variants) {
                let name = type_as_inline_cpp(&Type::Builtin(get_enum_variant_type(variants)));
                out.push_str(&indent_str);
                out.push_str(&format!("readData<{name}>(*(({name}*) &{base_addr}));\n"));
            } else {
                let variant_type_name =
                    type_as_inline_cpp(&Type::Builtin(get_enum_variant_type(variants)));

                out.push_str(&format!(
                    "{indent_str}readData<{variant_type_name}>({base_addr}.variant_id);\n"
                ));

                out.push_str(&indent_str);
                for (idx, variant) in variants.iter().enumerate() {
                    let variant_name = &variant.name;

                    if idx != 0 {
                        out.push_str("else ");
                    }
                    out.push_str(&format!("if ({base_addr}.variant_id == {idx}) {{\n"));

                    write_struct_decoder(
                        &variant.types,
                        indent + 1,
                        format!("{base_addr}.inner.{}_val", variant_name),
                        out,
                        discriminant,
                    );

                    out.push_str(&format!("{indent_str}}} "));
                }

                out.push_str("\n");
            }
        }
    }
}

fn write_struct_encoder(
    fields: &StructFields,
    indent: usize,
    val: String,
    out: &mut String,
    discriminant: &mut usize,
) {
    for field in fields {
        let name = &field.name;
        write_encoder(
            &field.ty,
            indent,
            format!("{val}.{name}"),
            out,
            discriminant,
        );
    }
}

fn write_encoder(
    ty: &Type,
    indent: usize,
    val: String,
    out: &mut String,
    discriminant: &mut usize,
) {
    let indent_str = "    ".repeat(indent);

    match ty {
        Type::Array(elem, size) => {
            let idx = format!("i_{discriminant}");
            *discriminant += 1;

            out.push_str(&format!(
                "{indent_str}for (int {idx} = 0; {idx} < {size}; {idx}++) {{\n"
            ));
            write_encoder(
                &elem,
                indent + 1,
                format!("{val}[{idx}]"),
                out,
                discriminant,
            );
            out.push_str(&format!("{indent_str}}}\n"));
        }
        Type::DynamicArray(elem) => {
            let idx = format!("i_{discriminant}");
            *discriminant += 1;

            out.push_str(&format!(
                "{indent_str}for (int {idx} = 0; {idx} < {val}.size(); {idx}++) {{\n"
            ));
            write_encoder(
                &elem,
                indent + 1,
                format!("{val}[{idx}]"),
                out,
                discriminant,
            );
            out.push_str(&format!("{indent_str}}}\n"));
        }
        Type::NamedType(name) => {
            out.push_str(&format!("{indent_str}write_{name}({val});\n"));
        }
        Type::Builtin(BuiltinType::Str) => {
            out.push_str(&format!("{indent_str}writeString({val});\n"));
        }
        Type::Builtin(_) => {
            let name = type_as_inline_cpp(ty);

            out.push_str(&format!("{indent_str}writeData<{name}>({val});\n"));
        }
        Type::Struct(fields) => {
            write_struct_encoder(&fields, indent, val, out, discriminant);
        }
        Type::Enum(variants) => {
            if is_basic_enum(&variants) {
                let name = type_as_inline_cpp(&Type::Builtin(get_enum_variant_type(variants)));
                out.push_str(&format!(
                    "{indent_str}writeData<{name}>(*(({name}*) &{val}));\n"
                ));
            } else {
                let variant_type_name =
                    type_as_inline_cpp(&Type::Builtin(get_enum_variant_type(variants)));

                out.push_str(&format!(
                    "{indent_str}writeData<{variant_type_name}>({val}.variant_id);\n"
                ));

                out.push_str(&indent_str);
                for (idx, variant) in variants.iter().enumerate() {
                    let variant_name = &variant.name;

                    if idx != 0 {
                        out.push_str("else ");
                    }
                    out.push_str(&format!("if ({val}.variant_id == {idx}) {{\n"));

                    write_struct_encoder(
                        &variant.types,
                        indent + 1,
                        format!("{val}.inner.{variant_name}_val"),
                        out,
                        discriminant,
                    );

                    out.push_str(&format!("{indent_str}}} "));
                }

                out.push_str("\n");
            }
        }
    }
}

//TODO: C++ Versioning
#[async_trait]
impl Language for CppLang {
    fn name(&self) -> &'static str {
        "C++"
    }

    fn id(&self) -> &'static str {
        "cpp"
    }

    fn extension(&self) -> &'static str {
        ".cpp"
    }

    fn generate(
        &self,
        game_interface: &gamedef::game_interface::GameInterface,
    ) -> super::files::ClientFiles {
        let mut res = ClientFiles::new();

        res.include_client_file("interact_lib.hpp", ".");

        let mut type_defs = String::new();

        type_defs.push_str("#include <vector>\n");
        type_defs.push_str("#include <string>\n");
        type_defs.push_str("#include <array>\n");
        type_defs.push_str("#include <stdint.h>\n");
        type_defs.push_str("\n\n");

        for (name, ty) in &game_interface.types {
            match ty {
                Type::Struct(fields) => {
                    type_defs.push_str("struct ");
                    type_defs.push_str(&name);
                    type_defs.push_str(" ");
                    type_defs.push_str(&struct_fields(&fields, true, ""));
                    type_defs.push_str(";\n\n");
                }
                Type::Enum(variants) => {
                    type_defs.push_str(&enum_decl(&variants, true, Some(name.clone())));
                    type_defs.push_str(";\n\n");
                }
                _ => {
                    type_defs.push_str(&format!(
                        "typedef {} {};\n\n",
                        type_as_inline_cpp(ty),
                        name
                    ));
                }
            }
        }

        res.add_file("game_types.h", type_defs, false);

        let mut function_decl = String::new();
        function_decl.push_str("#include \"game_types.h\"\n\n");

        for (function_name, signature) in &game_interface.functions {
            if let Some(ret) = &signature.ret {
                function_decl.push_str(&type_as_inline_cpp(ret));
            } else {
                function_decl.push_str("void");
            }

            function_decl.push(' ');

            function_decl.push_str(&function_name);
            function_decl.push('(');

            for (idx, (name, ty)) in signature.args.iter().enumerate() {
                function_decl.push_str(&type_as_inline_cpp(ty));

                let do_ref = match ty {
                    Type::Builtin(BuiltinType::Str) => true,
                    Type::Builtin(_) => false,
                    _ => true,
                };

                if do_ref {
                    function_decl.push('&');
                }

                function_decl.push(' ');

                function_decl.push_str(&name);

                if idx != signature.args.len() - 1 {
                    function_decl.push_str(", ");
                }
            }

            function_decl.push_str(");\n");
        }

        res.add_file("game.h", function_decl, false);

        let mut interactor = String::new();
        interactor.push_str("#include \"game.h\"\n");
        interactor.push_str("#include \"interact_lib.hpp\"\n");
        interactor.push_str("\n\n");

        for (name, ty) in &game_interface.types {
            interactor.push_str(&format!("void read_{}({}& addr) {{\n", name, name));

            let mut x = 0;
            write_decoder(ty, 1, "addr".to_string(), &mut interactor, &mut x);

            interactor.push_str("}\n\n");
        }

        for (name, ty) in &game_interface.types {
            interactor.push_str(&format!("void write_{}({}& x) {{\n", name, name));
            let mut x = 0;
            write_encoder(ty, 1, "x".to_string(), &mut interactor, &mut x);
            interactor.push_str("}\n\n");
        }

        interactor.push_str(
            "
int main(){
    while(true) {
        uint8_t func_id;
        readData<uint8_t>(func_id);
",
        );

        interactor.push_str("        ");
        for (idx, (function_name, signature)) in game_interface.functions.iter().enumerate() {
            if idx != 0 {
                interactor.push_str("else ");
            }

            interactor.push_str(&format!("if (func_id == {idx}) {{ // {function_name}\n"));

            for (name, ty) in &signature.args {
                let decl = type_as_inline_cpp(ty);
                interactor.push_str(&format!("            {decl} param_{name};\n"));
                let mut x = 0;
                write_decoder(ty, 3, format!("param_{name}"), &mut interactor, &mut x);
            }

            if signature.args.len() > 0 {
                interactor.push_str("            \n");
            }

            interactor.push_str("            ");

            if let Some(ret) = &signature.ret {
                let ret_str = type_as_inline_cpp(ret);
                interactor.push_str(&format!("{ret_str} ret = "));
            }

            interactor.push_str(&function_name);
            interactor.push_str("(");

            for (idx, (name, _ty)) in signature.args.iter().enumerate() {
                interactor.push_str(&format!("param_{name}"));
                if idx != signature.args.len() - 1 {
                    interactor.push_str(", ");
                }
            }

            interactor.push_str(");\n");

            if let Some(ret) = &signature.ret {
                let mut x = 0;
                write_encoder(ret, 3, "ret".to_string(), &mut interactor, &mut x);
                interactor.push_str("            flushStreams();\n");
            }

            interactor.push_str("        } ")
        }

        interactor.push_str("\n    }\n}");

        res.add_file("interactor.cpp", interactor, false);

        res
    }

    async fn prepare(
        &self,
        src: &str,
        out: &mut super::language::PreparedProgram,
        game_interface: &gamedef::game_interface::GameInterface,
        sandboxes: Pool<IsolateSandbox>,
    ) -> Result<(), String> {
        let sandbox = sandboxes.get().await.unwrap();

        let temp_folder = random_dir("./tmp");
        async_std::fs::write(format!("{}/{}", temp_folder, "agent.cpp"), src)
            .await
            .unwrap();

        make_public(&out.dir_as_string()).await;

        let mut compile_job: crate::isolate::sandbox::RunningJob = sandbox.launch(
            "/usr/bin/g++".to_string(),
            vec![
                //"-DVERBOSE_IO".to_string(),
                "-I/client_files/".to_string(),
                "-O2".to_string(),
                "-Wall".to_string(),
                "-std=c++20".to_string(),
                "-o".to_string(),
                "/out/agent.o".to_string(),
                "/client_files/interactor.cpp".to_string(),
                "/src/agent.cpp".to_string(),
            ],
            /*vec![
                (self.get_dir(&game_interface), "/client_files".to_string()),
                (out.dir_as_string(), "/out".to_string()),
                (temp_folder.clone(), "/src".to_string()),
                ("/usr/bin".to_string(), "/usr/bin".to_string())
            ],
            vec![],
            None,*/
            &LaunchOptions::new()
                .memory_limit_kb(512000)
                .time_limit_s(10.0)
                .max_processes(MaxProcessCount::Unlimited)
                .map_dir("/client_files", self.get_dir(game_interface))
                .add_mapping(DirMapping::named("/out", out.dir_as_string()).read_write())
                .map_dir("/src", temp_folder.clone())
                .map_dir("/usr/bin", "/usr/bin")
                .full_env()
        );

        let status = compile_job.wait().await.unwrap();


        if !status.success() {
            return Err(compile_job.stderr.read_as_string().await);
        }

        Ok(())
    }

    fn launch(
        &self,
        data_dir: &str,
        sandbox: &crate::isolate::sandbox::IsolateSandbox,
        _itf: &gamedef::game_interface::GameInterface,
    ) -> crate::isolate::sandbox::RunningJob {
        sandbox.launch(
            format!("{data_dir}/agent.o"), 
            vec![], 
            &LaunchOptions::new()
                .map_full(data_dir)
        )
    }
}
