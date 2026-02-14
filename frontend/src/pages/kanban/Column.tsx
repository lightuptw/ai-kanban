import React, { ReactNode } from "react";
import styled from "@emotion/styled";
import { useDroppable } from "@dnd-kit/core";
import { SortableContext, verticalListSortingStrategy } from "@dnd-kit/sortable";
import {
  Button,
  Typography as MuiTypography,
  Chip,
} from "@mui/material";
import { spacing } from "@mui/system";
import { Add as AddIcon } from "@mui/icons-material";

const Typography = styled(MuiTypography)(spacing);

const ColumnContainer = styled.div`
  display: flex;
  flex-direction: column;
  height: 100%;
  background: ${(props) => props.theme.palette.background.paper};
`;

const ColumnHeader = styled.div`
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: ${(props) => props.theme.spacing(2)} ${(props) => props.theme.spacing(3)};
  flex-shrink: 0;
`;

const ColumnTitle = styled(Typography)`
  display: flex;
  align-items: center;
  gap: ${(props) => props.theme.spacing(2)};
`;

const StageIndicator = styled.div<{ color: string }>`
  width: 4px;
  height: 24px;
  background: ${(props) => props.color};
  border-radius: 2px;
`;

const DropZone = styled.div<{ isOver: boolean; isEmpty: boolean }>`
  flex: 1;
  overflow-y: auto;
  background: ${(props) =>
    props.isOver ? props.theme.palette.action.hover : "transparent"};
  transition: background 0.2s ease;
  ${(props) =>
    props.isEmpty
      ? `
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100px;
  `
      : ""}
`;

const EmptyState = styled(Typography)`
  color: ${(props) => props.theme.palette.grey[500]};
  text-align: center;
`;

const AddButtonWrapper = styled.div`
  padding: 0;
  flex-shrink: 0;
`;

interface ColumnProps {
  id: string;
  title: string;
  color: string;
  items: any[];
  showAddButton?: boolean;
  onAddCard?: () => void;
  children: ReactNode;
}

export const Column: React.FC<ColumnProps> = ({
  id,
  title,
  color,
  items,
  showAddButton = false,
  onAddCard,
  children,
}) => {
  const { setNodeRef, isOver } = useDroppable({ id });
  const isEmpty = items.length === 0;

  return (
    <ColumnContainer>
      <ColumnHeader>
        <ColumnTitle variant="subtitle1">
          <StageIndicator color={color} />
          {title}
          <Chip label={items.length} size="small" />
        </ColumnTitle>
      </ColumnHeader>

      <SortableContext items={items.map((item) => item.id)} strategy={verticalListSortingStrategy}>
        <DropZone ref={setNodeRef} isOver={isOver} isEmpty={isEmpty}>
          {isEmpty ? (
            <EmptyState variant="body2">No cards</EmptyState>
          ) : (
            <div>{children}</div>
          )}
        </DropZone>
      </SortableContext>

      {showAddButton && (
        <AddButtonWrapper>
          <Button
            color="primary"
            variant="contained"
            fullWidth
            onClick={onAddCard}
            size="small"
          >
            <AddIcon />
            Add card
          </Button>
        </AddButtonWrapper>
      )}
    </ColumnContainer>
  );
};
