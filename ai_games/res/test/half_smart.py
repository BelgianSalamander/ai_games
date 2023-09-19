from game_types import Piece, Pos, BoardCell
from typing import List
import random

def cell_to_piece(cell: BoardCell) -> Piece:
    if cell == BoardCell.Nought:
        return Piece.Nought
    
    if cell == BoardCell.Cross:
        return Piece.Cross
    
    raise Exception("Invalid cell")

def piece_to_cell(piece: Piece) -> BoardCell:
    if piece == Piece.Nought:
        return BoardCell.Nought
    
    if piece == Piece.Cross:
        return BoardCell.Cross

    raise Exception("Invalid piece")

def opposite_piece(piece: Piece) -> Piece:
    if piece == Piece.Nought:
        return Piece.Cross
    
    if piece == Piece.Cross:
        return Piece.Nought
    
    raise Exception("Invalid piece")

def get_winner(board: List[List[BoardCell]]) -> Piece:
    for i in range(3):
        if board[i][0] != BoardCell.Empty and board[i][0] == board[i][1] == board[i][2]:
            return cell_to_piece(board[i][0])
        
        if board[0][i] != BoardCell.Empty and board[0][i] == board[1][i] == board[2][i]:
            return cell_to_piece(board[0][i])
        
    if board[0][0] != BoardCell.Empty and board[0][0] == board[1][1] == board[2][2]:
        return cell_to_piece(board[0][0])
    
    if board[0][2] != BoardCell.Empty and board[0][2] == board[1][1] == board[2][0]:
        return cell_to_piece(board[0][2])
    
    return None

def copy_board(board: List[List[BoardCell]]) -> List[List[BoardCell]]:
    return [[board[i][j] for j in range(3)] for i in range(3)]

def get_final_outcome(board: List[List[BoardCell]], piece: Piece) -> Piece:
    winner = get_winner(board)
    
    if winner is not None:
        return winner
    
    best_outcome = -1
    
    for i in range(3):
        for j in range(3):
            if board[i][j] == BoardCell.Empty:
                res = copy_board(board)
                res[i][j] = piece_to_cell(piece)
                
                outcome = get_final_outcome(res, Piece.Cross if piece == Piece.Nought else Piece.Nought)
                
                if outcome == piece:
                    return piece
                
                if outcome is None:
                    best_outcome = 0
    
    if best_outcome == -1:
        return opposite_piece(piece)
    
    if best_outcome == 0:
        return None
    
    return piece

def get_move(board: List[List[BoardCell]], piece: Piece) -> Pos:
    win_moves = []
    tie_moves = []
    loss_moves = []

    for i in range(3):
        for j in range(3):
            if board[i][j] == BoardCell.Empty:
                res = copy_board(board)
                res[i][j] = piece_to_cell(piece)
                
                outcome = get_final_outcome(res, opposite_piece(piece))
                
                if outcome == piece:
                    win_moves.append(Pos(i, j))
                elif outcome is None:
                    tie_moves.append(Pos(i, j))
                else:
                    loss_moves.append(Pos(i, j))
                
    if len(win_moves) > 0:
        return random.choice(win_moves)
    
    if len(tie_moves) > 0:
        return random.choice(tie_moves)
    
    return random.choice(loss_moves)