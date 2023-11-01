use std::time::Duration;

use async_trait::async_trait;
use proc_gamedef::make_server;

use crate::{isolate::sandbox::RunningJob, games::{await_seconds, Waiter}};

use super::{Game};

#[derive(serde::Serialize)]
pub struct TicTacToe;

make_server!("../../res/games/tic_tac_toe.game");

fn get_piece(cell: &BoardCell) -> Piece {
    match cell {
        BoardCell::Nought => Piece::Nought,
        BoardCell::Cross => Piece::Cross,
        BoardCell::Empty => panic!("Empty cell"),
    }
}

fn get_winner(grid: &Board) -> Option<Piece> {
    for i in 0..3 {
        if grid[i][0] == grid[i][1] && grid[i][1] == grid[i][2] && grid[i][0] != BoardCell::Empty {
            return Some(get_piece(&grid[i][0]));
        }
    }

    for i in 0..3 {
        if grid[0][i] == grid[1][i] && grid[1][i] == grid[2][i] && grid[0][i] != BoardCell::Empty {
            return Some(get_piece(&grid[0][i]));
        }
    }

    if grid[0][0] == grid[1][1] && grid[1][1] == grid[2][2] && grid[0][0] != BoardCell::Empty {
        return Some(get_piece(&grid[0][0]));
    }

    if grid[0][2] == grid[1][1] && grid[1][1] == grid[2][0] && grid[0][2] != BoardCell::Empty {
        return Some(get_piece(&grid[0][2]));
    }

    None
}

#[async_trait]
impl Game for TicTacToe {
    fn name(&self) -> &'static str {
        "Tic Tac Toe"
    }

    fn num_players(&self) -> usize {
        2
    }

    async fn run(&self, players: Vec<RunningJob>, min_delay: Option<Duration>) -> Vec<f32> {
        let mut waiter = Waiter::new(min_delay);
        let mut agents: Vec<_> = players.into_iter().map(|x| Agent::new(x)).collect();

        let mut grid = [[BoardCell::Empty; 3]; 3];

        let mut turn = 0;

        while turn < 9 {
            let player = turn % 2;
            let piece = if player == 0 {
                Piece::Cross
            } else {
                Piece::Nought
            };

            let m = match await_seconds(agents[player].get_move(&grid, &piece), 1.0).await {
                Ok(m) => m,
                Err(e) => {
                    agents[player].set_error(e);
                    for agent in agents {
                        agent.kill().await;
                    }

                    if player == 0 {
                        return vec![0.0, 1.0];
                    } else {
                        return vec![1.0, 0.0];
                    }
                }
            };

            waiter.wait().await;

            if m.row > 2 || m.col > 2 || grid[m.row as usize][m.col as usize] != BoardCell::Empty {
                agents[player].set_error(format!("Invalid Move ({}, {})", m.row, m.col));
                for agent in agents {
                    agent.kill().await;
                }

                if player == 0 {
                    return vec![0.0, 1.0];
                } else {
                    return vec![1.0, 0.0];
                }
            }

            grid[m.row as usize][m.col as usize] = if player == 0 {
                BoardCell::Cross
            } else {
                BoardCell::Nought
            };

            if let Some(winner) = get_winner(&grid) {
                for agent in agents {
                    agent.kill().await;
                }

                if winner == Piece::Nought {
                    return vec![0.0, 1.0];
                } else {
                    return vec![1.0, 0.0];
                }
            }

            turn += 1;
        }

        for agent in agents {
            agent.kill().await;
        }

        vec![0.5, 0.5]
    }
}

unsafe impl Send for TicTacToe {}
unsafe impl Sync for TicTacToe {}