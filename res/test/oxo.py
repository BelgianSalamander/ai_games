from game_types import Piece, Pos, BoardCell
from typing import List
import sys
import random
import time

def get_move(board: List[List[BoardCell]], piece: Piece) -> Pos:
    moves = []

    for i in range(3):
        for j in range(3):
            if board[i][j] == BoardCell.Empty:
                moves.append(Pos(i, j))
                
    return random.choice(moves)