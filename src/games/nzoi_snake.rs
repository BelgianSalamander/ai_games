use std::{collections::{HashMap, HashSet, VecDeque}, time::Duration};

use async_trait::async_trait;
use log::warn;
use proc_gamedef::make_server;
use rand::Rng;

use crate::{isolate::sandbox::RunningJob, games::{await_seconds, Waiter}, players::reporting::GameReporter};

use super::Game;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct NzoiSnake {
    size: (usize, usize),
    food: usize,

    snakes: Vec<Vec<(usize, usize)>>
}

impl NzoiSnake {
    fn rows(&self) -> usize {
        self.size.0
    }

    fn cols(&self) -> usize {
        self.size.1
    }
}

make_server!("res/games/nzoi_snake.game");

fn apply_move(p: Pos, m: Move) -> Pos {
    let Pos {row, col} = p;

    match m {
        Move::Up => Pos {
            row: row - 1,
            col
        },
        Move::Down => Pos {
            row: row + 1,
            col
        },
        Move::Left => Pos {
            row,
            col: col - 1
        },
        Move::Right => Pos {
            row,
            col: col + 1
        }
    }
}

#[async_trait]
impl Game for NzoiSnake {
    fn name(&self) -> &'static str {
        "Snake"
    }

    fn num_players(&self) -> usize {
        self.snakes.len()
    }

    async fn run(&self, players: &mut Vec<RunningJob>, min_delay: Option<Duration>, mut reporter: GameReporter) -> Vec<f32> {
        let mut waiter = Waiter::new(min_delay);
        let mut agents: Vec<_> = players.into_iter().map(|x| Agent::new(x)).collect();

        let mut grid: Vec<_> = (0..self.rows()).map(|_| vec![0i32; self.cols()]).collect();
        let mut snakes: Vec<_> = (0..self.num_players()).map(|_| VecDeque::new()).collect();
        let mut dead = vec![false; self.num_players()];
        let mut num_dead = 0;
        let mut scores = vec![0.0; self.num_players()];

        let size_data = (self.rows(), self.cols());
        reporter.update(&size_data, "dimensions").await;

        let mut turns_without_changes = 0;
        let mut turns_since_dead = 0;

        for (i, snake) in self.snakes.iter().enumerate() {
            for (row, col) in snake {
                grid[*row][*col] = i as i32 +1;
                snakes[i].push_back(Pos{row: *row as _, col: *col as _});
            }

            match agents[i].init(&(i as i32 + 1), self.rows() as u32, self.cols() as u32, self.num_players() as u32).await {
                Err(e) => {
                    warn!("Snake init error!");
                    reporter.update(&(i+1), "init_error").await;
                    agents[i].set_error(e.to_string());
                    dead[i] = true;
                    num_dead += 1;
                },
                Ok(_) => {}
            }
        }

        reporter.update(&grid, "grid").await;

        while num_dead < self.num_players() {
            if num_dead >= self.num_players() - 1 {
                turns_since_dead += 1;
                if turns_since_dead > 10 {
                    break;
                }
            }

            let prev_scores = scores.clone();
            let prev_grid = grid.clone();

            let mut num_food_on_board = 0;

            for i in 0..self.rows() {
                for j in 0..self.cols() {
                    if grid[i][j] == -1 {
                        num_food_on_board += 1;
                    }
                }
            }

            {
                let mut rng = rand::thread_rng();

                let mut num_tries = 100;

                while num_food_on_board < self.food && num_tries > 0 {
                    num_tries -= 1;

                    let row = rng.gen_range(0..self.rows());
                    let col = rng.gen_range(0..self.cols());

                    if grid[row][col] == 0{
                        num_food_on_board += 1;
                        grid[row][col] = -1;
                        turns_without_changes = 0;
                    }
                }
            }

            let futures = agents.iter_mut().enumerate().filter_map(|(i, agent)| 
                if dead[i] {
                    None
                } else {
                    Some(await_seconds(agent.get_move(&grid, &snakes[i].back().unwrap()), 1.0))
                }
            );

            let moves = futures::future::join_all(futures).await;

            waiter.wait().await;

            let alive_players: Vec<_> = (0..self.num_players()).filter(|x| !dead[*x]).collect();
            let mut new_positions = vec![];
            let mut to_kill = vec![];

            for (i, res) in alive_players.iter().zip(moves) {
                match res {
                    Err(e) => {
                        warn!("Snake crashed! {:?}", e);
                        reporter.update(&(i+1), "player_error").await;
                        agents[*i].set_error(e.clone());
                        dead[*i] = true;
                        num_dead += 1;

                        to_kill.push(*i);
                    },
                    Ok(m) => {
                        let curr_head = snakes[*i].back().unwrap();
                        let new_pos = apply_move(*curr_head, m);

                        if new_pos.row < 0 || new_pos.col < 0 || new_pos.row >= self.rows() as i32 || new_pos.col >= self.cols() as i32 {
                            reporter.update(&(i+1), "wall_crash").await;
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
                    reporter.update(&(i+1), "head_butt").await;
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
                    reporter.update(&(i+1), "snake_crash").await;
                    scores[(grid[pos.row as usize][pos.col as usize] - 1) as usize] += 1.0;
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

                for i in 0..self.num_players() {
                    if !dead[i] {
                        scores[i] += 5.0;
                    }
                }
            }

            // reporter.update(&grid, "grid").await;
            // reporter.update(&scores, "scores").await;

            let mut changes: HashMap<i32, Vec<(usize, usize)>> = HashMap::new();

            for i in 0..self.rows() {
                for j in 0..self.cols() {
                    if prev_grid[i][j] != grid[i][j] {
                        let new_val = grid[i][j];

                        if !changes.contains_key(&new_val) {
                            changes.insert(new_val, vec![]);
                        }

                        changes.get_mut(&new_val).unwrap().push((i, j));
                    }
                }
            }

            let mut update_data = vec![];

            for (val, positions) in changes {
                let mut sub = vec![val];

                for (i, j) in positions {
                    sub.push(i as i32);
                    sub.push(j as i32);
                }

                update_data.push(sub);
            }

            reporter.update(&update_data, "upd").await;

            let score_changes: Vec<_> = scores.iter().zip(prev_scores).map(|(a, b)| a - b).collect();
            if score_changes.iter().any(|&x: &f32| x.abs() > 0.0001) {
                reporter.update(&score_changes, "scr").await;
            }

            turns_without_changes += 1;

            if turns_without_changes > 50 {
                break;
            }
        }

        println!("Killing snakes!");

        for agent in agents {
            agent.kill().await;
        }

        scores
    }
}

unsafe impl Send for NzoiSnake {}
unsafe impl Sync for NzoiSnake {}