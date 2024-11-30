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

#[cfg(test)]
mod tests {
    use deadpool::unmanaged::Pool;
    use gamedef::parser::parse_game_interface;
    use log::info;
    use proc_gamedef::make_server;
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha20Rng;

    use crate::{games::await_seconds, isolate::sandbox::IsolateSandbox, langs::language::Language};
    use super::{cpp::CppLang, language::PreparedProgram, python::Python};

    make_server!("test_res/games/ser_test.game");


    #[test]
    fn test_serialisation() {
        env_logger::Builder::from_env(
            env_logger::Env::default()
                .default_filter_or("ai_games=debug")
                .default_write_style_or("always")
        )
        .format_timestamp(None)
        .format_module_path(false)
        .init();
        
        let itf_path = "test_res/games/ser_test.game";
        let itf = std::fs::read_to_string(itf_path).unwrap();
        let itf = parse_game_interface(&itf, "ser_test".to_string()).unwrap();

        let tests: Vec<(Box<dyn Language>, &str)> = vec![
            (Box::new(CppLang), "test_res/ser_test_agents/agent.cpp"),
            (Box::new(Python), "test_res/ser_test_agents/agent.py"),
        ];

        let sandboxes = Pool::new(1);
        pollster::block_on(sandboxes.add(pollster::block_on(IsolateSandbox::new(1)))).unwrap();

        let mut rng = ChaCha20Rng::seed_from_u64(8);
        let chars: Vec<_> = "ABCEEFGHOJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789".chars().collect();

        for (lang, agent_file) in tests {
            let client_files = lang.prepare_files(&itf);

            let src = std::fs::read_to_string(agent_file).unwrap();
            let mut program = PreparedProgram::new();
            
            pollster::block_on(lang.prepare(&src, &mut program, &itf, sandboxes.clone())).unwrap();

            let mut sandbox = pollster::block_on(sandboxes.get()).unwrap();
            pollster::block_on(sandbox.initialize());
            let mut job: crate::isolate::sandbox::RunningJob = lang.launch(&program.dir_as_string(), &sandbox, &itf);
            job.stderr.freeze();
            job._metafile.freeze();

            let mut agent = Agent::new(&mut job);

            let mut whole = vec![];

            for i in 0..1000 {
                let a = rng.r#gen();
                let b = rng.r#gen();
                let c = rng.r#gen();
                let d = rng.r#gen();
                let e = rng.r#gen();
                let f = rng.r#gen();
                let g = rng.r#gen();
                let h = rng.r#gen();
                let i = rng.r#gen();
                let j = rng.r#gen();
                let k = rng.gen_bool(0.5);
                
                let l_length = rng.gen_range(0..100);
                let l = (0..l_length).map(|_| chars[rng.gen_range(0..chars.len())]).collect();

                let s = BigStruct {
                    a, b, c, d, e, f, g, h, i, j, k, l
                };

                assert_eq!(pollster::block_on(await_seconds(agent.get_a(&s), 0.1)).unwrap(), s.a);
                assert_eq!(pollster::block_on(await_seconds(agent.get_b(&s), 0.1)).unwrap(), s.b);
                assert_eq!(pollster::block_on(await_seconds(agent.get_c(&s), 0.1)).unwrap(), s.c);
                assert_eq!(pollster::block_on(await_seconds(agent.get_d(&s), 0.1)).unwrap(), s.d);
                assert_eq!(pollster::block_on(await_seconds(agent.get_e(&s), 0.1)).unwrap(), s.e);
                assert_eq!(pollster::block_on(await_seconds(agent.get_f(&s), 0.1)).unwrap(), s.f);
                assert_eq!(pollster::block_on(await_seconds(agent.get_g(&s), 0.1)).unwrap(), s.g);
                assert_eq!(pollster::block_on(await_seconds(agent.get_h(&s), 0.1)).unwrap(), s.h);
                assert_eq!(pollster::block_on(await_seconds(agent.get_i(&s), 0.1)).unwrap(), s.i);
                assert_eq!(pollster::block_on(await_seconds(agent.get_j(&s), 0.1)).unwrap(), s.j);
                assert_eq!(pollster::block_on(await_seconds(agent.get_k(&s), 0.1)).unwrap(), s.k);
                assert_eq!(pollster::block_on(await_seconds(agent.get_l(&s), 0.1)).unwrap(), s.l);
                assert_eq!(pollster::block_on(await_seconds(agent.list_test(&whole), 0.1)).unwrap(), whole);

                whole.push(s);
            }

            pollster::block_on(agent.kill());
            pollster::block_on(sandbox.cleanup());
        }
    }
}