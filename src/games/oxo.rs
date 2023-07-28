use async_trait::async_trait;
use proc_gamedef::make_server;

use crate::isolate::sandbox::RunningJob;

use super::{Game, PlayerResult};

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
    fn name() -> &'static str {
        "Tic Tac Toe"
    }

    fn num_players() -> usize {
        2
    }

    async fn run(players: Vec<RunningJob>) -> Vec<PlayerResult> {
        let mut agents: Vec<_> = players.into_iter().map(|x| Agent::new(x)).collect();

        let mut grid = [[BoardCell::Empty; 3]; 3];

        let mut turn = 0;

        while turn < 9 {
            let player = turn % 2;

            let m = match agents[player].get_move(&grid).await {
                Ok(m) => m,
                Err(e) => {
                    if player == 0 {
                        return vec![PlayerResult {
                            score: 0.0,
                            error: Some(format!("Client Error: {:?}", e)),
                        }, PlayerResult {
                            score: 1.0,
                            error: None,
                        }];
                    } else {
                        return vec![PlayerResult {
                            score: 1.0,
                            error: None,
                        }, PlayerResult {
                            score: 0.0,
                            error: Some(format!("Client Error: {:?}", e)),
                        }];
                    }
                }
            };

            if m.row > 2 || m.col > 2 || grid[m.row as usize][m.col as usize] != BoardCell::Empty {
                if player == 0 {
                    return vec![PlayerResult {
                        score: 0.0,
                        error: Some(format!("Invalid Move ({}, {})", m.row, m.col)),
                    }, PlayerResult {
                        score: 1.0,
                        error: None,
                    }];
                } else {
                    return vec![PlayerResult {
                        score: 1.0,
                        error: None,
                    }, PlayerResult {
                        score: 0.0,
                        error: Some(format!("Invalid Move ({}, {})", m.row, m.col)),
                    }];
                }
            }

            grid[m.row as usize][m.col as usize] = if player == 0 {
                BoardCell::Nought
            } else {
                BoardCell::Cross
            };

            if let Some(winner) = get_winner(&grid) {
                if winner == Piece::Nought {
                    return vec![PlayerResult {
                        score: 1.0,
                        error: None,
                    }, PlayerResult {
                        score: 0.0,
                        error: None,
                    }];
                } else {
                    return vec![PlayerResult {
                        score: 0.0,
                        error: None,
                    }, PlayerResult {
                        score: 1.0,
                        error: None,
                    }];
                }
            }

            turn += 1;

            println!("Grid:");

            for i in 0..3 {
                for j in 0..3 {
                    print!("{}", match grid[i][j] {
                        BoardCell::Nought => "O",
                        BoardCell::Cross => "X",
                        BoardCell::Empty => " ",
                    });
                }

                println!();
            }
        }

        vec![PlayerResult {
            score: 0.5,
            error: None,
        }, PlayerResult {
            score: 0.5,
            error: None,
        }]
    }
}