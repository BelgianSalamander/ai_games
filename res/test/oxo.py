from game_types import Piece, Pos, BoardCell
from typing import List
import sys
import random
import time

def get_move(board: List[List[BoardCell]]) -> Pos:
    if random.randint(0, 5) == 0:
        exit(0)
        
    sys.stderr.write('Read: ' + str(board) + '\n')
    sys.stderr.flush()
    for i in range(3):
        for j in range(3):
            if board[i][j] == BoardCell.Empty:
                sys.stderr.write('Write: ' + str(Pos(i, j)) + '\n')
                sys.stderr.flush()
                return Pos(i, j)