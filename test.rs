pub mod nzoi_snake {
    use std::{time::Duration, collections::VecDeque};
    use async_trait::async_trait;
    use log::warn;
    use proc_gamedef::make_server;
    use rand::Rng;
    use crate::{
        isolate::sandbox::RunningJob, games::{await_seconds, Waiter},
        players::reporting::GameReporter,
    };
    use super::Game;
    pub struct NzoiSnake {
        size: (usize, usize),
        food: usize,
        snakes: Vec<Vec<(usize, usize)>>,
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for NzoiSnake {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                let mut __serde_state = _serde::Serializer::serialize_struct(
                    __serializer,
                    "NzoiSnake",
                    false as usize + 1 + 1 + 1,
                )?;
                _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "size",
                    &self.size,
                )?;
                _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "food",
                    &self.food,
                )?;
                _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "snakes",
                    &self.snakes,
                )?;
                _serde::ser::SerializeStruct::end(__serde_state)
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de> for NzoiSnake {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                #[allow(non_camel_case_types)]
                #[doc(hidden)]
                enum __Field {
                    __field0,
                    __field1,
                    __field2,
                    __ignore,
                }
                #[doc(hidden)]
                struct __FieldVisitor;
                impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                    type Value = __Field;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "field identifier",
                        )
                    }
                    fn visit_u64<__E>(
                        self,
                        __value: u64,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            0u64 => _serde::__private::Ok(__Field::__field0),
                            1u64 => _serde::__private::Ok(__Field::__field1),
                            2u64 => _serde::__private::Ok(__Field::__field2),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                    fn visit_str<__E>(
                        self,
                        __value: &str,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            "size" => _serde::__private::Ok(__Field::__field0),
                            "food" => _serde::__private::Ok(__Field::__field1),
                            "snakes" => _serde::__private::Ok(__Field::__field2),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                    fn visit_bytes<__E>(
                        self,
                        __value: &[u8],
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            b"size" => _serde::__private::Ok(__Field::__field0),
                            b"food" => _serde::__private::Ok(__Field::__field1),
                            b"snakes" => _serde::__private::Ok(__Field::__field2),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                }
                impl<'de> _serde::Deserialize<'de> for __Field {
                    #[inline]
                    fn deserialize<__D>(
                        __deserializer: __D,
                    ) -> _serde::__private::Result<Self, __D::Error>
                    where
                        __D: _serde::Deserializer<'de>,
                    {
                        _serde::Deserializer::deserialize_identifier(
                            __deserializer,
                            __FieldVisitor,
                        )
                    }
                }
                #[doc(hidden)]
                struct __Visitor<'de> {
                    marker: _serde::__private::PhantomData<NzoiSnake>,
                    lifetime: _serde::__private::PhantomData<&'de ()>,
                }
                impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                    type Value = NzoiSnake;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "struct NzoiSnake",
                        )
                    }
                    #[inline]
                    fn visit_seq<__A>(
                        self,
                        mut __seq: __A,
                    ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::SeqAccess<'de>,
                    {
                        let __field0 = match _serde::de::SeqAccess::next_element::<
                            (usize, usize),
                        >(&mut __seq)? {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(
                                    _serde::de::Error::invalid_length(
                                        0usize,
                                        &"struct NzoiSnake with 3 elements",
                                    ),
                                );
                            }
                        };
                        let __field1 = match _serde::de::SeqAccess::next_element::<
                            usize,
                        >(&mut __seq)? {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(
                                    _serde::de::Error::invalid_length(
                                        1usize,
                                        &"struct NzoiSnake with 3 elements",
                                    ),
                                );
                            }
                        };
                        let __field2 = match _serde::de::SeqAccess::next_element::<
                            Vec<Vec<(usize, usize)>>,
                        >(&mut __seq)? {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(
                                    _serde::de::Error::invalid_length(
                                        2usize,
                                        &"struct NzoiSnake with 3 elements",
                                    ),
                                );
                            }
                        };
                        _serde::__private::Ok(NzoiSnake {
                            size: __field0,
                            food: __field1,
                            snakes: __field2,
                        })
                    }
                    #[inline]
                    fn visit_map<__A>(
                        self,
                        mut __map: __A,
                    ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::MapAccess<'de>,
                    {
                        let mut __field0: _serde::__private::Option<(usize, usize)> = _serde::__private::None;
                        let mut __field1: _serde::__private::Option<usize> = _serde::__private::None;
                        let mut __field2: _serde::__private::Option<
                            Vec<Vec<(usize, usize)>>,
                        > = _serde::__private::None;
                        while let _serde::__private::Some(__key) = _serde::de::MapAccess::next_key::<
                            __Field,
                        >(&mut __map)? {
                            match __key {
                                __Field::__field0 => {
                                    if _serde::__private::Option::is_some(&__field0) {
                                        return _serde::__private::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field("size"),
                                        );
                                    }
                                    __field0 = _serde::__private::Some(
                                        _serde::de::MapAccess::next_value::<
                                            (usize, usize),
                                        >(&mut __map)?,
                                    );
                                }
                                __Field::__field1 => {
                                    if _serde::__private::Option::is_some(&__field1) {
                                        return _serde::__private::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field("food"),
                                        );
                                    }
                                    __field1 = _serde::__private::Some(
                                        _serde::de::MapAccess::next_value::<usize>(&mut __map)?,
                                    );
                                }
                                __Field::__field2 => {
                                    if _serde::__private::Option::is_some(&__field2) {
                                        return _serde::__private::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field("snakes"),
                                        );
                                    }
                                    __field2 = _serde::__private::Some(
                                        _serde::de::MapAccess::next_value::<
                                            Vec<Vec<(usize, usize)>>,
                                        >(&mut __map)?,
                                    );
                                }
                                _ => {
                                    let _ = _serde::de::MapAccess::next_value::<
                                        _serde::de::IgnoredAny,
                                    >(&mut __map)?;
                                }
                            }
                        }
                        let __field0 = match __field0 {
                            _serde::__private::Some(__field0) => __field0,
                            _serde::__private::None => {
                                _serde::__private::de::missing_field("size")?
                            }
                        };
                        let __field1 = match __field1 {
                            _serde::__private::Some(__field1) => __field1,
                            _serde::__private::None => {
                                _serde::__private::de::missing_field("food")?
                            }
                        };
                        let __field2 = match __field2 {
                            _serde::__private::Some(__field2) => __field2,
                            _serde::__private::None => {
                                _serde::__private::de::missing_field("snakes")?
                            }
                        };
                        _serde::__private::Ok(NzoiSnake {
                            size: __field0,
                            food: __field1,
                            snakes: __field2,
                        })
                    }
                }
                #[doc(hidden)]
                const FIELDS: &'static [&'static str] = &["size", "food", "snakes"];
                _serde::Deserializer::deserialize_struct(
                    __deserializer,
                    "NzoiSnake",
                    FIELDS,
                    __Visitor {
                        marker: _serde::__private::PhantomData::<NzoiSnake>,
                        lifetime: _serde::__private::PhantomData,
                    },
                )
            }
        }
    };
    impl NzoiSnake {
        fn rows(&self) -> usize {
            self.size.0
        }
        fn cols(&self) -> usize {
            self.size.1
        }
    }
    pub type GridCell = i32;
    pub struct Pos {
        row: i32,
        col: i32,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Pos {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "Pos",
                "row",
                &self.row,
                "col",
                &&self.col,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Pos {
        #[inline]
        fn clone(&self) -> Pos {
            let _: ::core::clone::AssertParamIsClone<i32>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Pos {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Pos {
        #[inline]
        fn eq(&self, other: &Pos) -> bool {
            self.row == other.row && self.col == other.col
        }
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for Pos {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                let mut __serde_state = _serde::Serializer::serialize_struct(
                    __serializer,
                    "Pos",
                    false as usize + 1 + 1,
                )?;
                _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "row",
                    &self.row,
                )?;
                _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "col",
                    &self.col,
                )?;
                _serde::ser::SerializeStruct::end(__serde_state)
            }
        }
    };
    #[automatically_derived]
    impl ::core::marker::Copy for Pos {}
    #[automatically_derived]
    impl ::core::cmp::Eq for Pos {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {
            let _: ::core::cmp::AssertParamIsEq<i32>;
        }
    }
    pub enum Move {
        Up,
        Down,
        Left,
        Right,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Move {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(
                f,
                match self {
                    Move::Up => "Up",
                    Move::Down => "Down",
                    Move::Left => "Left",
                    Move::Right => "Right",
                },
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Move {
        #[inline]
        fn clone(&self) -> Move {
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Move {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Move {
        #[inline]
        fn eq(&self, other: &Move) -> bool {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            __self_discr == __arg1_discr
        }
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for Move {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                match *self {
                    Move::Up => {
                        _serde::Serializer::serialize_unit_variant(
                            __serializer,
                            "Move",
                            0u32,
                            "Up",
                        )
                    }
                    Move::Down => {
                        _serde::Serializer::serialize_unit_variant(
                            __serializer,
                            "Move",
                            1u32,
                            "Down",
                        )
                    }
                    Move::Left => {
                        _serde::Serializer::serialize_unit_variant(
                            __serializer,
                            "Move",
                            2u32,
                            "Left",
                        )
                    }
                    Move::Right => {
                        _serde::Serializer::serialize_unit_variant(
                            __serializer,
                            "Move",
                            3u32,
                            "Right",
                        )
                    }
                }
            }
        }
    };
    #[automatically_derived]
    impl ::core::marker::Copy for Move {}
    #[automatically_derived]
    impl ::core::cmp::Eq for Move {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    fn serialize_GridCell(value: &GridCell, out: &mut Vec<u8>) {
        out.extend(value.to_le_bytes());
    }
    fn serialize_Pos(value: &Pos, out: &mut Vec<u8>) {
        out.extend(value.row.to_le_bytes());
        out.extend(value.col.to_le_bytes());
    }
    fn serialize_Move(value: &Move, out: &mut Vec<u8>) {
        match value {
            Move::Up => {
                out.extend(&[0x00]);
            }
            Move::Down => {
                out.extend(&[0x01]);
            }
            Move::Left => {
                out.extend(&[0x02]);
            }
            Move::Right => {
                out.extend(&[0x03]);
            }
        }
    }
    async fn deserialize_GridCell(
        instance: &mut crate::isolate::sandbox::RunningJob,
    ) -> Result<GridCell, async_std::io::Error> {
        Ok(instance.read_i32().await?)
    }
    async fn deserialize_Pos(
        instance: &mut crate::isolate::sandbox::RunningJob,
    ) -> Result<Pos, async_std::io::Error> {
        Ok(Pos {
            row: instance.read_i32().await?,
            col: instance.read_i32().await?,
        })
    }
    async fn deserialize_Move(
        instance: &mut crate::isolate::sandbox::RunningJob,
    ) -> Result<Move, async_std::io::Error> {
        Ok(
            match instance.read_u8().await? {
                0u8 => Move::Up,
                1u8 => Move::Down,
                2u8 => Move::Left,
                3u8 => Move::Right,
                _ => {
                    return Err(
                        async_std::io::Error::new(
                            async_std::io::ErrorKind::InvalidData,
                            "Invalid enum variant",
                        ),
                    );
                }
            },
        )
    }
    struct Agent<'a> {
        instance: &'a mut crate::isolate::sandbox::RunningJob,
    }
    impl<'a> Agent<'a> {
        pub fn new(instance: &'a mut crate::isolate::sandbox::RunningJob) -> Self {
            Self { instance }
        }
        pub async fn init(
            &mut self,
            snake_id: &GridCell,
            num_rows: u32,
            num_cols: u32,
            num_snakes: u32,
        ) -> Result<(), async_std::io::Error> {
            let mut out_bytes: Vec<u8> = Vec::new();
            out_bytes.extend(&[0]);
            serialize_GridCell(&snake_id, (&mut out_bytes));
            (&mut out_bytes).extend(num_rows.to_le_bytes());
            (&mut out_bytes).extend(num_cols.to_le_bytes());
            (&mut out_bytes).extend(num_snakes.to_le_bytes());
            self.instance.write(&out_bytes).await?;
            Ok(())
        }
        pub async fn get_move(
            &mut self,
            grid: &Vec<Vec<GridCell>>,
            head: &Pos,
        ) -> Result<Move, async_std::io::Error> {
            let mut out_bytes: Vec<u8> = Vec::new();
            out_bytes.extend(&[1]);
            (&mut out_bytes).extend(&(grid.len() as u32).to_le_bytes());
            for x in (grid).iter() {
                (&mut out_bytes).extend(&(x.len() as u32).to_le_bytes());
                for x in (x).iter() {
                    serialize_GridCell(&x, (&mut out_bytes));
                }
            }
            serialize_Pos(&head, (&mut out_bytes));
            self.instance.write(&out_bytes).await?;
            Ok(deserialize_Move((&mut self.instance)).await?)
        }
        pub async fn kill(mut self) {
            match self.instance.kill().await {
                Ok(_) => {}
                Err(e) => {
                    let lvl = ::log::Level::Error;
                    if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                        ::log::__private_api::log(
                            format_args!("Failed to kill sandbox: {0:?}", e),
                            lvl,
                            &(
                                "ai_games::games::nzoi_snake",
                                "ai_games::games::nzoi_snake",
                                ::log::__private_api::loc(),
                            ),
                            (),
                        );
                    }
                }
            }
        }
        pub fn set_error(&mut self, error: String) {
            self.instance.set_error(error)
        }
    }
    fn apply_move(p: Pos, m: Move) -> Pos {
        let Pos { row, col } = p;
        match m {
            Move::Up => Pos { row: row - 1, col },
            Move::Down => Pos { row: row + 1, col },
            Move::Left => Pos { row, col: col - 1 },
            Move::Right => Pos { row, col: col + 1 },
        }
    }
    impl Game for NzoiSnake {
        fn name(&self) -> &'static str {
            "Snake"
        }
        fn num_players(&self) -> usize {
            self.snakes.len()
        }
        #[allow(
            elided_named_lifetimes,
            clippy::async_yields_async,
            clippy::diverging_sub_expression,
            clippy::let_unit_value,
            clippy::needless_arbitrary_self_type,
            clippy::no_effect_underscore_binding,
            clippy::shadow_same,
            clippy::type_complexity,
            clippy::type_repetition_in_bounds,
            clippy::used_underscore_binding
        )]
        fn run<'life0, 'life1, 'async_trait>(
            &'life0 self,
            players: &'life1 mut Vec<RunningJob>,
            min_delay: Option<Duration>,
            reporter: GameReporter,
        ) -> ::core::pin::Pin<
            Box<
                dyn ::core::future::Future<
                    Output = Vec<f32>,
                > + ::core::marker::Send + 'async_trait,
            >,
        >
        where
            'life0: 'async_trait,
            'life1: 'async_trait,
            Self: 'async_trait,
        {
            Box::pin(async move {
                if let ::core::option::Option::Some(__ret) = ::core::option::Option::None::<
                    Vec<f32>,
                > {
                    #[allow(unreachable_code)] return __ret;
                }
                let __self = self;
                let min_delay = min_delay;
                let mut reporter = reporter;
                let __ret: Vec<f32> = {
                    let mut waiter = Waiter::new(min_delay);
                    let mut agents: Vec<_> = players
                        .into_iter()
                        .map(|x| Agent::new(x))
                        .collect();
                    let mut grid: Vec<_> = (0..__self.rows())
                        .map(|_| ::alloc::vec::from_elem(0i32, __self.cols()))
                        .collect();
                    let mut snakes: Vec<_> = (0..__self.num_players())
                        .map(|_| VecDeque::new())
                        .collect();
                    let mut dead = ::alloc::vec::from_elem(false, __self.num_players());
                    let mut num_dead = 0;
                    let mut scores = ::alloc::vec::from_elem(0.0, __self.num_players());
                    let size_data = (__self.rows(), __self.cols());
                    reporter.update(&size_data, "dimensions").await;
                    let mut turns_without_changes = 0;
                    let mut turns_since_dead = 0;
                    for (i, snake) in __self.snakes.iter().enumerate() {
                        for (row, col) in snake {
                            grid[*row][*col] = i as i32 + 1;
                            snakes[i]
                                .push_back(Pos {
                                    row: *row as _,
                                    col: *col as _,
                                });
                        }
                        match agents[i]
                            .init(
                                &(i as i32 + 1),
                                __self.rows() as u32,
                                __self.cols() as u32,
                                __self.num_players() as u32,
                            )
                            .await
                        {
                            Err(e) => {
                                {
                                    let lvl = ::log::Level::Warn;
                                    if lvl <= ::log::STATIC_MAX_LEVEL
                                        && lvl <= ::log::max_level()
                                    {
                                        ::log::__private_api::log(
                                            format_args!("Snake init error!"),
                                            lvl,
                                            &(
                                                "ai_games::games::nzoi_snake",
                                                "ai_games::games::nzoi_snake",
                                                ::log::__private_api::loc(),
                                            ),
                                            (),
                                        );
                                    }
                                };
                                reporter.update(&(i + 1), "init_error").await;
                                agents[i].set_error(e.to_string());
                                dead[i] = true;
                                num_dead += 1;
                            }
                            Ok(_) => {}
                        }
                    }
                    while num_dead < __self.num_players() {
                        if num_dead >= __self.num_players() - 1 {
                            turns_since_dead += 1;
                            if turns_since_dead > 10 {
                                break;
                            }
                        }
                        let mut num_food_on_board = 0;
                        for i in 0..__self.rows() {
                            for j in 0..__self.cols() {
                                if grid[i][j] == -1 {
                                    num_food_on_board += 1;
                                }
                            }
                        }
                        {
                            let mut rng = rand::thread_rng();
                            let mut num_tries = 100;
                            while num_food_on_board < __self.food && num_tries > 0 {
                                num_tries -= 1;
                                let row = rng.gen_range(0..__self.rows());
                                let col = rng.gen_range(0..__self.cols());
                                if grid[row][col] == 0 {
                                    num_food_on_board += 1;
                                    grid[row][col] = -1;
                                    turns_without_changes = 0;
                                }
                            }
                        }
                        let futures = agents
                            .iter_mut()
                            .enumerate()
                            .filter_map(|(i, agent)| {
                                if dead[i] {
                                    None
                                } else {
                                    Some(
                                        await_seconds(
                                            agent.get_move(&grid, &snakes[i].back().unwrap()),
                                            1.0,
                                        ),
                                    )
                                }
                            });
                        let moves = futures::future::join_all(futures).await;
                        waiter.wait().await;
                        let alive_players: Vec<_> = (0..__self.num_players())
                            .filter(|x| !dead[*x])
                            .collect();
                        let mut new_positions = ::alloc::vec::Vec::new();
                        let mut to_kill = ::alloc::vec::Vec::new();
                        for (i, res) in alive_players.iter().zip(moves) {
                            match res {
                                Err(e) => {
                                    {
                                        let lvl = ::log::Level::Warn;
                                        if lvl <= ::log::STATIC_MAX_LEVEL
                                            && lvl <= ::log::max_level()
                                        {
                                            ::log::__private_api::log(
                                                format_args!("Snake crashed! {0:?}", e),
                                                lvl,
                                                &(
                                                    "ai_games::games::nzoi_snake",
                                                    "ai_games::games::nzoi_snake",
                                                    ::log::__private_api::loc(),
                                                ),
                                                (),
                                            );
                                        }
                                    };
                                    reporter.update(&(i + 1), "player_error").await;
                                    agents[*i].set_error(e.clone());
                                    dead[*i] = true;
                                    num_dead += 1;
                                    to_kill.push(*i);
                                }
                                Ok(m) => {
                                    let curr_head = snakes[*i].back().unwrap();
                                    let new_pos = apply_move(*curr_head, m);
                                    if new_pos.row < 0 || new_pos.col < 0
                                        || new_pos.row >= __self.rows() as i32
                                        || new_pos.col >= __self.cols() as i32
                                    {
                                        reporter.update(&(i + 1), "wall_crash").await;
                                        dead[*i] = true;
                                        num_dead += 1;
                                        to_kill.push(*i);
                                    } else {
                                        new_positions.push((*i, new_pos));
                                    }
                                }
                            }
                        }
                        for i in 0..new_positions.len() {
                            let mut head_crash = false;
                            for j in 0..new_positions.len() {
                                if i != j && new_positions[i].1 == new_positions[j].1 {
                                    head_crash = true;
                                    break;
                                }
                            }
                            let (snake, pos) = new_positions[i];
                            if head_crash {
                                reporter.update(&(i + 1), "head_butt").await;
                                dead[snake] = true;
                                num_dead += 1;
                                to_kill.push(snake);
                            } else if grid[pos.row as usize][pos.col as usize] != -1 {
                                if let Some(p) = snakes[snake].pop_front() {
                                    grid[p.row as usize][p.col as usize] = 0;
                                }
                            } else {
                                scores[snake] += 1.0;
                            }
                        }
                        for i in 0..new_positions.len() {
                            let (snake, pos) = new_positions[i];
                            if dead[snake] {
                                continue;
                            }
                            if grid[pos.row as usize][pos.col as usize] > 0 {
                                reporter.update(&(i + 1), "snake_crash").await;
                                scores[(grid[pos.row as usize][pos.col as usize] - 1)
                                    as usize] += 1.0;
                                dead[snake] = true;
                                num_dead += 1;
                                to_kill.push(snake);
                            } else {
                                grid[pos.row as usize][pos.col as usize] = (snake + 1) as _;
                                snakes[snake].push_back(pos);
                            }
                        }
                        for snake in to_kill {
                            turns_without_changes = 0;
                            let mut rng = rand::thread_rng();
                            while !snakes[snake].is_empty() {
                                let p = snakes[snake].pop_front().unwrap();
                                if rng.gen_range(0.0..1.0) < 0.3 {
                                    grid[p.row as usize][p.col as usize] = -1;
                                } else {
                                    grid[p.row as usize][p.col as usize] = 0;
                                }
                            }
                            for i in 0..__self.num_players() {
                                if !dead[i] {
                                    scores[i] += 5.0;
                                }
                            }
                        }
                        reporter.update(&grid, "grid").await;
                        reporter.update(&scores, "scores").await;
                        turns_without_changes += 1;
                        if turns_without_changes > 50 {
                            break;
                        }
                    }
                    {
                        ::std::io::_print(format_args!("Killing snakes!\n"));
                    };
                    for agent in agents {
                        agent.kill().await;
                    }
                    scores
                };
                #[allow(unreachable_code)] __ret
            })
        }
    }
    unsafe impl Send for NzoiSnake {}
    unsafe impl Sync for NzoiSnake {}
}
