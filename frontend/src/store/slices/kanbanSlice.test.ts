import { describe, it, expect } from "vitest";
import kanbanReducer, {
  optimisticMoveCard,
  revertMoveCard,
  setSelectedCard,
} from "./kanbanSlice";
import type { Card } from "../../types/kanban";

const mockCard: Card = {
  id: "test-card-1",
  title: "Test Card",
  description: "",
  stage: "backlog",
  position: 1000,
  priority: "medium",
  working_directory: ".",
  plan_path: null,
  ai_session_id: null,
  ai_status: "idle",
  ai_progress: {},
  linked_documents: [],
  created_at: "2026-02-14T00:00:00Z",
  updated_at: "2026-02-14T00:00:00Z",
  subtasks: [],
  labels: [],
  comments: [],
};

describe("kanbanSlice", () => {
  it("should set selected card", () => {
    const initialState = {
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
    };

    const state = kanbanReducer(initialState, setSelectedCard("card-123"));
    expect(state.selectedCardId).toBe("card-123");
  });

  it("should optimistically move card between columns", () => {
    const initialState = {
      columns: {
        backlog: [mockCard],
        plan: [],
        todo: [],
        in_progress: [],
        review: [],
        done: [],
      },
      loading: false,
      error: null,
      selectedCardId: null,
    };

    const state = kanbanReducer(
      initialState,
      optimisticMoveCard({
        cardId: "test-card-1",
        fromStage: "backlog",
        toStage: "plan",
        position: 2000,
      })
    );

    expect(state.columns.backlog).toHaveLength(0);
    expect(state.columns.plan).toHaveLength(1);
    expect(state.columns.plan[0].id).toBe("test-card-1");
    expect(state.columns.plan[0].stage).toBe("plan");
    expect(state.columns.plan[0].position).toBe(2000);
  });

  it("should revert card move on error", () => {
    const movedCard = { ...mockCard, stage: "plan" as const, position: 2000 };
    const initialState = {
      columns: {
        backlog: [],
        plan: [movedCard],
        todo: [],
        in_progress: [],
        review: [],
        done: [],
      },
      loading: false,
      error: null,
      selectedCardId: null,
    };

    const state = kanbanReducer(
      initialState,
      revertMoveCard({
        cardId: "test-card-1",
        fromStage: "backlog",
        toStage: "plan",
      })
    );

    expect(state.columns.plan).toHaveLength(0);
    expect(state.columns.backlog).toHaveLength(1);
    expect(state.columns.backlog[0].id).toBe("test-card-1");
    expect(state.columns.backlog[0].stage).toBe("backlog");
  });

  it("should sort cards by position after move", () => {
    const card1 = { ...mockCard, id: "card-1", position: 1000 };
    const card2 = { ...mockCard, id: "card-2", position: 3000 };
    
    const initialState = {
      columns: {
        backlog: [card1],
        plan: [card2],
        todo: [],
        in_progress: [],
        review: [],
        done: [],
      },
      loading: false,
      error: null,
      selectedCardId: null,
    };

    const state = kanbanReducer(
      initialState,
      optimisticMoveCard({
        cardId: "card-1",
        fromStage: "backlog",
        toStage: "plan",
        position: 2000,
      })
    );

    expect(state.columns.plan).toHaveLength(2);
    expect(state.columns.plan[0].position).toBe(2000);
    expect(state.columns.plan[1].position).toBe(3000);
  });
});
