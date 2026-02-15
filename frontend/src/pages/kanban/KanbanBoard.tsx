import React, { useState, useEffect, useCallback, useRef } from "react";
import { useDispatch, useSelector } from "react-redux";
import styled from "@emotion/styled";
import { Helmet } from "react-helmet-async";
import {
  DndContext,
  DragOverlay,
  closestCenter,
  pointerWithin,
  rectIntersection,
  getFirstCollision,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  DragStartEvent,
  DragEndEvent,
  DragOverEvent,
  CollisionDetection,
  UniqueIdentifier,
} from "@dnd-kit/core";
import { arrayMove, sortableKeyboardCoordinates } from "@dnd-kit/sortable";
import {
  CircularProgress,
  Box,
  Typography as MuiTypography,
} from "@mui/material";
import { spacing } from "@mui/system";
import { Column } from "./Column";
import { KanbanCard } from "./KanbanCard";
import { AppDispatch, RootState } from "../../redux/store";
import { fetchBoard, fetchBoards, optimisticMoveCard, revertMoveCard, moveCard, setSelectedCard, createCard } from "../../store/slices/kanbanSlice";
import { CardDetailDialog } from "./CardDetailDialog";
import type { Stage } from "../../types/kanban";
import { STAGE_COLORS } from "../../constants/stageColors";

const Typography = styled(MuiTypography)(spacing);

interface CardItem {
  id: string;
  title: string;
  badges?: string[];
  notifications?: number;
  avatars?: number[];
}

function KanbanBoard() {
  const dispatch = useDispatch<AppDispatch>();
  const { columns, loading, error, activeBoardId } = useSelector((state: RootState) => state.kanban);
  const { selectedCardId } = useSelector((state: RootState) => state.kanban);

  const [activeId, setActiveId] = useState<string | null>(null);
  const [dragOrigin, setDragOrigin] = useState<{ stage: Stage; position: number } | null>(null);
  const lastOverId = useRef<UniqueIdentifier | null>(null);

  const columnIds = Object.keys(columns);

  const collisionDetectionStrategy: CollisionDetection = useCallback(
    (args) => {
      if (activeId && columnIds.includes(activeId)) {
        return closestCenter({ ...args, droppableContainers: args.droppableContainers.filter((c) => columnIds.includes(c.id as string)) });
      }

      const pointerCollisions = pointerWithin(args);
      const collisions = pointerCollisions.length > 0 ? pointerCollisions : rectIntersection(args);

      let overId = getFirstCollision(collisions, "id");

      if (overId != null) {
        if (columnIds.includes(overId as string)) {
          const columnItems = columns[overId as Stage] || [];
          if (columnItems.length > 0) {
            const closestInColumn = closestCenter({
              ...args,
              droppableContainers: args.droppableContainers.filter(
                (c) => c.id !== overId && columnItems.some((item) => item.id === c.id)
              ),
            });
            overId = getFirstCollision(closestInColumn, "id") ?? overId;
          }
        }
        lastOverId.current = overId;
        return [{ id: overId }];
      }

      if (lastOverId.current) {
        return [{ id: lastOverId.current }];
      }

      return [];
    },
    [activeId, columns, columnIds]
  );

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 8,
      },
    }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates })
  );

  const findColumnForCard = useCallback((cardId: string): Stage | null => {
    for (const [columnId, items] of Object.entries(columns)) {
      if (items.some((item) => item.id === cardId)) {
        return columnId as Stage;
      }
    }
    return null;
  }, [columns]);

  useEffect(() => {
    dispatch(fetchBoards());
  }, [dispatch]);

  useEffect(() => {
    if (activeBoardId) {
      dispatch(fetchBoard(activeBoardId));
    }
  }, [dispatch, activeBoardId]);

  if (loading) {
    return (
      <Box display="flex" justifyContent="center" alignItems="center" minHeight="400px">
        <CircularProgress />
      </Box>
    );
  }

  if (error) {
    return (
      <Box display="flex" justifyContent="center" alignItems="center" minHeight="400px">
        <Typography color="error">{error}</Typography>
      </Box>
    );
  }

  const handleDragStart = (event: DragStartEvent) => {
    const cardId = event.active.id as string;
    setActiveId(cardId);
    const stage = findColumnForCard(cardId);
    if (stage) {
      const card = columns[stage].find((c) => c.id === cardId);
      setDragOrigin({ stage, position: card?.position ?? 0 });
    }
  };

  const handleDragOver = (event: DragOverEvent) => {
    const { active, over } = event;
    if (!over) return;

    const activeId = active.id as string;
    const overId = over.id as string;

    const activeColumn = findColumnForCard(activeId);
    if (!activeColumn) return;

    const isOverColumn = Object.keys(columns).includes(overId);
    const overColumn: Stage | null = isOverColumn
      ? (overId as Stage)
      : findColumnForCard(overId);

    if (!overColumn) return;
    if (activeColumn === overColumn) return;

    const overItems = columns[overColumn];
    let newPosition: number;

    if (isOverColumn || overItems.length === 0) {
      newPosition = overItems.length > 0
        ? overItems[overItems.length - 1].position + 1000
        : 1000;
    } else {
      const overIndex = overItems.findIndex((item) => item.id === overId);
      if (overIndex <= 0) {
        newPosition = overItems.length > 0
          ? Math.max(1, Math.floor(overItems[0].position / 2))
          : 1000;
      } else {
        const before = overItems[overIndex - 1];
        const after = overItems[overIndex];
        newPosition = Math.floor((before.position + after.position) / 2);
      }
    }

    dispatch(optimisticMoveCard({
      cardId: activeId,
      fromStage: activeColumn,
      toStage: overColumn,
      position: newPosition,
    }));
  };

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    setActiveId(null);

    if (!over || !dragOrigin) {
      setDragOrigin(null);
      return;
    }

    const activeId = active.id as string;
    const overId = over.id as string;

    const currentColumn = findColumnForCard(activeId);
    if (!currentColumn) {
      setDragOrigin(null);
      return;
    }

    const isOverColumn = Object.keys(columns).includes(overId);
    const overColumn: Stage | null = isOverColumn
      ? (overId as Stage)
      : findColumnForCard(overId);

    if (!overColumn) {
      setDragOrigin(null);
      return;
    }

    const currentItems = columns[currentColumn];
    const activeIndex = currentItems.findIndex((item) => item.id === activeId);

    if (currentColumn === overColumn && !isOverColumn) {
      const overIndex = currentItems.findIndex((item) => item.id === overId);
      if (activeIndex !== overIndex && overIndex !== -1) {
        let newPosition: number;
        const sorted = [...currentItems].sort((a, b) => a.position - b.position);
        const sortedOverIndex = sorted.findIndex((item) => item.id === overId);

        if (sortedOverIndex === 0) {
          newPosition = Math.max(1, Math.floor(sorted[0].position / 2));
        } else if (sortedOverIndex >= sorted.length - 1) {
          newPosition = sorted[sorted.length - 1].position + 1000;
        } else {
          const before = sorted[sortedOverIndex - 1];
          const after = sorted[sortedOverIndex];
          if (before.id === activeId) {
            const afterAfter = sorted[sortedOverIndex + 1];
            newPosition = afterAfter
              ? Math.floor((after.position + afterAfter.position) / 2)
              : after.position + 1000;
          } else {
            newPosition = Math.floor((before.position + after.position) / 2);
          }
        }

        dispatch(moveCard({
          id: activeId,
          data: { stage: currentColumn, position: newPosition },
        })).unwrap().catch((error) => {
          console.error("Failed to reorder card:", error);
          dispatch(fetchBoard(activeBoardId || undefined));
        });

        setDragOrigin(null);
        return;
      }
    }

    if (currentColumn !== dragOrigin.stage || currentColumn !== overColumn) {
      const card = currentItems.find((c) => c.id === activeId);
      const finalPosition = card?.position ?? 1000;

      dispatch(moveCard({
        id: activeId,
        data: { stage: currentColumn, position: finalPosition },
      })).unwrap().catch((error) => {
        console.error("Failed to move card:", error);
        dispatch(revertMoveCard({
          cardId: activeId,
          fromStage: dragOrigin.stage,
          toStage: currentColumn,
        }));
      });
    }

    setDragOrigin(null);
  };

  const activeItem = activeId ? Object.values(columns).flat().find((item) => item.id === activeId) : null;

  const handleCardClick = (cardId: string) => {
    dispatch(setSelectedCard(cardId));
  };

  const handleCloseDialog = () => {
    dispatch(setSelectedCard(null));
  };

  const handleAddCard = () => {
    dispatch(createCard({
      title: "New Card",
      description: "",
      priority: "medium",
      board_id: activeBoardId || undefined,
    }));
  };

  return (
    <React.Fragment>
      <Helmet title="Kanban Board" />
      <Box sx={{ overflowX: 'auto', overflowY: 'hidden', height: 'calc(100vh - 100px)' }}>
        <DndContext sensors={sensors} collisionDetection={collisionDetectionStrategy} onDragStart={handleDragStart} onDragOver={handleDragOver} onDragEnd={handleDragEnd}>
          <Box sx={{ display: 'flex', flexWrap: 'nowrap', height: '100%', gap: '5px' }}>
            <Box sx={{ width: 250, minWidth: 250, height: '100%' }}>
              <Column id="backlog" title="Backlog" color={STAGE_COLORS.backlog} items={columns.backlog} showAddButton={true} onAddCard={handleAddCard}>
                {columns.backlog.map((item) => (<KanbanCard key={item.id} {...item} aiStatus={item.ai_status} onClick={() => handleCardClick(item.id)} />))}
              </Column>
            </Box>
            <Box sx={{ width: 250, minWidth: 250, height: '100%' }}>
              <Column id="plan" title="Plan" color={STAGE_COLORS.plan} items={columns.plan}>
                {columns.plan.map((item) => (<KanbanCard key={item.id} {...item} aiStatus={item.ai_status} onClick={() => handleCardClick(item.id)} />))}
              </Column>
            </Box>
            <Box sx={{ width: 250, minWidth: 250, height: '100%' }}>
              <Column id="todo" title="Todo" color={STAGE_COLORS.todo} items={columns.todo}>
                {columns.todo.map((item) => (<KanbanCard key={item.id} {...item} aiStatus={item.ai_status} onClick={() => handleCardClick(item.id)} />))}
              </Column>
            </Box>
            <Box sx={{ width: 250, minWidth: 250, height: '100%' }}>
              <Column id="in_progress" title="In Progress" color={STAGE_COLORS.in_progress} items={columns.in_progress}>
                {columns.in_progress.map((item) => (<KanbanCard key={item.id} {...item} aiStatus={item.ai_status} onClick={() => handleCardClick(item.id)} />))}
              </Column>
            </Box>
            <Box sx={{ width: 250, minWidth: 250, height: '100%' }}>
              <Column id="review" title="Review" color={STAGE_COLORS.review} items={columns.review}>
                {columns.review.map((item) => (<KanbanCard key={item.id} {...item} aiStatus={item.ai_status} onClick={() => handleCardClick(item.id)} />))}
              </Column>
            </Box>
            <Box sx={{ width: 250, minWidth: 250, height: '100%' }}>
              <Column id="done" title="Done" color={STAGE_COLORS.done} items={columns.done}>
                {columns.done.map((item) => (<KanbanCard key={item.id} {...item} aiStatus={item.ai_status} onClick={() => handleCardClick(item.id)} />))}
              </Column>
            </Box>
          </Box>
          <DragOverlay>{activeItem ? <KanbanCard {...activeItem} /> : null}</DragOverlay>
        </DndContext>
      </Box>
      <CardDetailDialog open={!!selectedCardId} onClose={handleCloseDialog} cardId={selectedCardId || ""} />
    </React.Fragment>
  );
}

export default KanbanBoard;
