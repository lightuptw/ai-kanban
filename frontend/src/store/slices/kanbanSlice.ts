import { createSlice, createAsyncThunk, PayloadAction } from "@reduxjs/toolkit";
import type { Card, BoardResponse, CreateCardRequest, UpdateCardRequest, MoveCardRequest, Stage, Board, CreateBoardRequest, UpdateBoardRequest, ReorderBoardRequest } from "../../types/kanban";
import { api } from "../../services/api";

interface KanbanState {
  columns: Record<Stage, Card[]>;
  loading: boolean;
  error: string | null;
  selectedCardId: string | null;
  boards: Board[];
  activeBoardId: string | null;
  boardsLoading: boolean;
}

const initialState: KanbanState = {
  columns: {
    backlog: [],
    plan: [],
    todo: [],
    in_progress: [],
    review: [],
    done: [],
  },
  loading: false,
  error: null,
  selectedCardId: null,
  boards: [],
  activeBoardId: null,
  boardsLoading: false,
};

export const fetchBoard = createAsyncThunk("kanban/fetchBoard", async (boardId?: string) => {
  return await api.getBoard(boardId);
});

export const fetchBoards = createAsyncThunk("kanban/fetchBoards", async () => {
  return await api.listBoards();
});

export const createBoard = createAsyncThunk("kanban/createBoard", async (data: CreateBoardRequest) => {
  return await api.createBoard(data);
});

export const updateBoard = createAsyncThunk(
  "kanban/updateBoard",
  async ({ id, data }: { id: string; data: UpdateBoardRequest }) => {
    return await api.updateBoard(id, data);
  }
);

export const reorderBoard = createAsyncThunk(
  "kanban/reorderBoard",
  async ({ id, data }: { id: string; data: ReorderBoardRequest }) => {
    return await api.reorderBoard(id, data);
  }
);

export const deleteBoard = createAsyncThunk(
  "kanban/deleteBoard",
  async (id: string, { getState, dispatch }) => {
    await api.deleteBoard(id);
    const state = (getState() as { kanban: KanbanState }).kanban;
    const remaining = state.boards.filter((b) => b.id !== id);
    if (state.activeBoardId === id && remaining.length > 0) {
      dispatch(setActiveBoard(remaining[0].id));
    }
    return id;
  }
);

export const createCard = createAsyncThunk("kanban/createCard", async (data: CreateCardRequest) => {
  return await api.createCard(data);
});

export const updateCard = createAsyncThunk(
  "kanban/updateCard",
  async ({ id, data }: { id: string; data: UpdateCardRequest }) => {
    return await api.updateCard(id, data);
  }
);

export const moveCard = createAsyncThunk(
  "kanban/moveCard",
  async ({ id, data }: { id: string; data: MoveCardRequest }) => {
    return await api.moveCard(id, data);
  }
);

export const deleteCard = createAsyncThunk("kanban/deleteCard", async (id: string) => {
  await api.deleteCard(id);
  return id;
});

const kanbanSlice = createSlice({
  name: "kanban",
  initialState,
  reducers: {
    setSelectedCard: (state, action: PayloadAction<string | null>) => {
      state.selectedCardId = action.payload;
    },
    setActiveBoard: (state, action: PayloadAction<string>) => {
      state.activeBoardId = action.payload;
    },
    optimisticMoveCard: (
      state,
      action: PayloadAction<{ cardId: string; fromStage: Stage; toStage: Stage; position: number }>
    ) => {
      const { cardId, fromStage, toStage, position } = action.payload;
      const cardIndex = state.columns[fromStage].findIndex((c) => c.id === cardId);
      if (cardIndex === -1) return;

      const [card] = state.columns[fromStage].splice(cardIndex, 1);
      card.stage = toStage;
      card.position = position;
      state.columns[toStage].push(card);
      state.columns[toStage].sort((a, b) => a.position - b.position);
    },
    revertMoveCard: (
      state,
      action: PayloadAction<{ cardId: string; fromStage: Stage; toStage: Stage }>
    ) => {
      const { cardId, fromStage, toStage } = action.payload;
      const cardIndex = state.columns[toStage].findIndex((c) => c.id === cardId);
      if (cardIndex === -1) return;

      const [card] = state.columns[toStage].splice(cardIndex, 1);
      card.stage = fromStage;
      state.columns[fromStage].push(card);
      state.columns[fromStage].sort((a, b) => a.position - b.position);
    },
    updateCardFromSSE: (state, action: PayloadAction<Card>) => {
      const card = action.payload;
      const stage = card.stage as Stage;
      const index = state.columns[stage].findIndex((c) => c.id === card.id);
      if (index !== -1) {
        state.columns[stage][index] = card;
      } else {
        state.columns[stage].push(card);
        state.columns[stage].sort((a, b) => a.position - b.position);
      }
    },
    updateCardAiStatus: (
      state,
      action: PayloadAction<{
        cardId: string;
        status: string;
        progress?: any;
        stage?: string;
        ai_session_id?: string | null;
      }>
    ) => {
      const { cardId, status, progress, stage: newStage, ai_session_id } = action.payload;
      for (const stageName of Object.keys(state.columns) as Stage[]) {
        const index = state.columns[stageName].findIndex((c) => c.id === cardId);
        if (index !== -1) {
          const card = state.columns[stageName][index];
          card.ai_status = status;
          if (progress !== undefined) {
            card.ai_progress = typeof progress === "string" ? progress : JSON.stringify(progress);
          }
          if (ai_session_id !== undefined) {
            card.ai_session_id = ai_session_id;
          }
          // If stage changed, move card to new stage
          if (newStage && newStage !== stageName) {
            state.columns[stageName].splice(index, 1);
            card.stage = newStage as Stage;
            state.columns[newStage as Stage].push(card);
            state.columns[newStage as Stage].sort((a, b) => a.position - b.position);
          }
          break;
        }
      }
    },
    moveCardInStore: (
      state,
      action: PayloadAction<{ cardId: string; fromStage: string; toStage: string }>
    ) => {
      const { cardId, fromStage, toStage } = action.payload;
      const from = fromStage as Stage;
      const to = toStage as Stage;
      const index = state.columns[from]?.findIndex((c) => c.id === cardId);
      if (index !== undefined && index !== -1) {
        const [card] = state.columns[from].splice(index, 1);
        card.stage = to;
        state.columns[to].push(card);
        state.columns[to].sort((a, b) => a.position - b.position);
      }
    },
    removeCardFromSSE: (state, action: PayloadAction<string>) => {
      const cardId = action.payload;
      for (const stage of Object.keys(state.columns) as Stage[]) {
        state.columns[stage] = state.columns[stage].filter((c) => c.id !== cardId);
      }
    },
    updateBoardFromSSE: (state, action: PayloadAction<Board>) => {
      const board = action.payload;
      const idx = state.boards.findIndex((b) => b.id === board.id);
      if (idx !== -1) {
        state.boards[idx] = board;
      } else {
        state.boards.push(board);
        state.boards.sort((a, b) => a.position - b.position);
      }
    },
    removeBoardFromSSE: (state, action: PayloadAction<string>) => {
      const boardId = action.payload;
      state.boards = state.boards.filter((b) => b.id !== boardId);
      if (state.activeBoardId === boardId && state.boards.length > 0) {
        state.activeBoardId = state.boards[0].id;
      }
    },
    optimisticReorderBoards: (state, action: PayloadAction<Board[]>) => {
      state.boards = action.payload;
    },
  },
  extraReducers: (builder) => {
    builder
      .addCase(fetchBoard.pending, (state) => {
        state.loading = true;
        state.error = null;
      })
      .addCase(fetchBoard.fulfilled, (state, action: PayloadAction<BoardResponse>) => {
        state.loading = false;
        state.columns = action.payload;
      })
      .addCase(fetchBoard.rejected, (state, action) => {
        state.loading = false;
        state.error = action.error.message || "Failed to fetch board";
      })
      .addCase(createCard.fulfilled, (state, action: PayloadAction<Card>) => {
        const card = action.payload;
        const stage = card.stage as Stage;
        state.columns[stage].push(card);
        state.columns[stage].sort((a, b) => a.position - b.position);
      })
      .addCase(updateCard.fulfilled, (state, action: PayloadAction<Card>) => {
        const card = action.payload;
        const stage = card.stage as Stage;
        const index = state.columns[stage].findIndex((c) => c.id === card.id);
        if (index !== -1) {
          state.columns[stage][index] = card;
        }
      })
      .addCase(moveCard.fulfilled, (state, action: PayloadAction<Card>) => {
        const card = action.payload;
        for (const stage of Object.keys(state.columns) as Stage[]) {
          state.columns[stage] = state.columns[stage].filter((c) => c.id !== card.id);
        }
        const newStage = card.stage as Stage;
        state.columns[newStage].push(card);
        state.columns[newStage].sort((a, b) => a.position - b.position);
      })
      .addCase(deleteCard.fulfilled, (state, action: PayloadAction<string>) => {
        const cardId = action.payload;
        for (const stage of Object.keys(state.columns) as Stage[]) {
          state.columns[stage] = state.columns[stage].filter((c) => c.id !== cardId);
        }
      })
      .addCase(fetchBoards.pending, (state) => {
        state.boardsLoading = true;
      })
      .addCase(fetchBoards.fulfilled, (state, action: PayloadAction<Board[]>) => {
        state.boardsLoading = false;
        state.boards = action.payload;
        if (!state.activeBoardId && action.payload.length > 0) {
          state.activeBoardId = action.payload[0].id;
        }
      })
      .addCase(fetchBoards.rejected, (state) => {
        state.boardsLoading = false;
      })
      .addCase(createBoard.fulfilled, (state, action: PayloadAction<Board>) => {
        state.boards.unshift(action.payload);
      })
      .addCase(updateBoard.fulfilled, (state, action: PayloadAction<Board>) => {
        const idx = state.boards.findIndex((b) => b.id === action.payload.id);
        if (idx !== -1) {
          state.boards[idx] = action.payload;
        }
      })
      .addCase(reorderBoard.fulfilled, (state, action: PayloadAction<Board>) => {
        const idx = state.boards.findIndex((b) => b.id === action.payload.id);
        if (idx !== -1) {
          state.boards[idx] = action.payload;
        }
      })
      .addCase(deleteBoard.fulfilled, (state, action: PayloadAction<string>) => {
        state.boards = state.boards.filter((b) => b.id !== action.payload);
      });
  },
});

export const { setSelectedCard, setActiveBoard, optimisticMoveCard, revertMoveCard, updateCardFromSSE, removeCardFromSSE, optimisticReorderBoards, updateCardAiStatus, moveCardInStore, updateBoardFromSSE, removeBoardFromSSE } =
  kanbanSlice.actions;

export default kanbanSlice.reducer;
