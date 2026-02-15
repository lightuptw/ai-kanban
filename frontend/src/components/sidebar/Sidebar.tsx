import React, { useState } from "react";
import styled from "@emotion/styled";
import { NavLink } from "react-router-dom";
import { useSelector, useDispatch } from "react-redux";
import {
  DndContext,
  closestCenter,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  DragEndEvent,
} from "@dnd-kit/core";
import {
  SortableContext,
  sortableKeyboardCoordinates,
  verticalListSortingStrategy,
  useSortable,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { restrictToVerticalAxis, restrictToParentElement } from "@dnd-kit/modifiers";

import { green } from "@mui/material/colors";

import {
  Box,
  Chip,
  Drawer as MuiDrawer,
  ListItemButton,
  List,
  ListItemText,
  IconButton,
  InputBase,
  Typography,
} from "@mui/material";
import {
  Add as AddIcon,
  Dashboard as DashboardIcon,
  DragIndicator as DragIcon,
  Settings as SettingsIcon,
} from "@mui/icons-material";

import lightupLogo from "../../vendor/lightup_logo.png";
import { SidebarItemsType } from "../../types/sidebar";
import Footer from "./SidebarFooter";
import { RootState, AppDispatch } from "../../redux/store";
import { setActiveBoard, createBoard, reorderBoard, optimisticReorderBoards } from "../../store/slices/kanbanSlice";
import type { Board } from "../../types/kanban";
import BoardSettingsDialog from "../../pages/kanban/BoardSettingsDialog";

const Drawer = styled(MuiDrawer)`
  border-right: 0;

  > div {
    border-right: 0;
  }
`;

const Brand = styled(ListItemButton)<{
  component?: React.ReactNode;
  to?: string;
}>`
  font-size: ${(props) => props.theme.typography.h5.fontSize};
  font-weight: ${(props) => props.theme.typography.fontWeightMedium};
  color: ${(props) => props.theme.sidebar.header.color};
  background-color: ${(props) => props.theme.sidebar.header.background};
  font-family: ${(props) => props.theme.typography.fontFamily};
  min-height: 56px;
  padding-left: ${(props) => props.theme.spacing(6)};
  padding-right: ${(props) => props.theme.spacing(6)};
  justify-content: center;
  cursor: pointer;
  flex-grow: 0;

  ${(props) => props.theme.breakpoints.up("sm")} {
    min-height: 64px;
  }

  &:hover {
    background-color: ${(props) => props.theme.sidebar.header.background};
  }
`;

const BrandIcon = styled.img`
  margin-right: ${(props) => props.theme.spacing(2)};
  width: 32px;
  height: 32px;
`;

const BrandChip = styled(Chip)`
  background-color: ${green[700]};
  border-radius: 5px;
  color: ${(props) => props.theme.palette.common.white};
  font-size: 55%;
  height: 18px;
  margin-left: 2px;
  margin-top: -16px;
  padding: 3px 0;

  span {
    padding-left: ${(props) => props.theme.spacing(1.375)};
    padding-right: ${(props) => props.theme.spacing(1.375)};
  }
`;

const BoardList = styled.div`
  flex-grow: 1;
  overflow-y: auto;
  background-color: ${(props) => props.theme.sidebar.background};
  border-right: 1px solid rgba(0, 0, 0, 0.12);
`;

const BoardSectionTitle = styled(Typography)`
  color: ${(props) => props.theme.sidebar.color};
  font-size: ${(props) => props.theme.typography.caption.fontSize};
  padding: ${(props) => props.theme.spacing(4)}
    ${(props) => props.theme.spacing(7)} ${(props) => props.theme.spacing(1)};
  opacity: 0.4;
  text-transform: uppercase;
`;

const BoardItem = styled(ListItemButton, {
  shouldForwardProp: (prop) => prop !== "active",
})<{ active?: boolean }>`
  padding-left: ${(props) => props.theme.spacing(7)};
  padding-top: ${(props) => props.theme.spacing(1.5)};
  padding-bottom: ${(props) => props.theme.spacing(1.5)};
  color: ${(props) => props.theme.sidebar.color};
  background: ${(props) =>
    props.active ? "rgba(255, 255, 255, 0.08)" : "transparent"};
  font-weight: ${(props) => (props.active ? 600 : 400)};

  &:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .settings-icon {
    opacity: 0;
    transition: opacity 0.2s ease;
  }

  &:hover .settings-icon {
    opacity: 0.6;
  }

  svg {
    color: ${(props) => props.theme.sidebar.color};
    opacity: 0.5;
    font-size: 20px;
    width: 20px;
    height: 20px;
    margin-right: ${(props) => props.theme.spacing(3)};
  }
`;

interface SortableBoardItemProps {
  board: Board;
  isActive: boolean;
  onSelect: () => void;
  onSettings?: (board: Board) => void;
}

const SortableBoardItem: React.FC<SortableBoardItemProps> = ({
  board,
  isActive,
  onSelect,
  onSettings,
}) => {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: board.id });
  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  };

  return (
    <div ref={setNodeRef} style={style}>
      <BoardItem active={isActive} onClick={onSelect}>
        <Box {...attributes} {...listeners} sx={{ display: 'flex', alignItems: 'center', cursor: 'grab', mr: 1 }}>
          <DragIcon sx={{ fontSize: 16, opacity: 0.4 }} />
        </Box>
        <DashboardIcon />
        <ListItemText
          primary={board.name}
          primaryTypographyProps={{ fontSize: "0.875rem", noWrap: true }}
        />
        <IconButton
          size="small"
          className="settings-icon"
          onClick={(event) => {
            event.stopPropagation();
            onSettings?.(board);
          }}
          sx={{
            ml: 1,
            color: "inherit",
            "& .MuiSvgIcon-root": { marginRight: 0, fontSize: 18 },
          }}
        >
          <SettingsIcon />
        </IconButton>
      </BoardItem>
    </div>
  );
};

const AddBoardRow = styled.div`
  display: flex;
  align-items: center;
  padding: ${(props) => props.theme.spacing(1)} ${(props) => props.theme.spacing(5)};
`;

const AddBoardInput = styled(InputBase)`
  flex: 1;
  color: ${(props) => props.theme.sidebar.color};
  font-size: 0.85rem;
  & .MuiInputBase-input {
    padding: 4px 8px;
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 4px;
  }
`;

export type SidebarProps = {
  PaperProps: {
    style: {
      width: number;
    };
  };
  variant?: "permanent" | "persistent" | "temporary";
  open?: boolean;
  onClose?: () => void;
  items: {
    title: string;
    pages: SidebarItemsType[];
  }[];
  showFooter?: boolean;
};

const Sidebar: React.FC<SidebarProps> = ({
  items,
  showFooter = true,
  ...rest
}) => {
  const dispatch = useDispatch<AppDispatch>();
  const { boards, activeBoardId } = useSelector((state: RootState) => state.kanban);
  const [addingBoard, setAddingBoard] = useState(false);
  const [newBoardName, setNewBoardName] = useState("");
  const [settingsBoard, setSettingsBoard] = useState<Board | null>(null);

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates })
  );

  const handleAddBoard = () => {
    const trimmed = newBoardName.trim();
    if (trimmed) {
      dispatch(createBoard({ name: trimmed }));
      setNewBoardName("");
      setAddingBoard(false);
    }
  };

  const handleBoardDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    if (!over || active.id === over.id) return;

    const oldIndex = boards.findIndex((b) => b.id === active.id);
    const newIndex = boards.findIndex((b) => b.id === over.id);
    if (oldIndex === -1 || newIndex === -1) return;

    const reordered = [...boards];
    const [moved] = reordered.splice(oldIndex, 1);
    reordered.splice(newIndex, 0, moved);

    let newPosition: number;
    if (newIndex === 0) {
      newPosition = Math.max(1, Math.floor(reordered[1]?.position ?? 1000) / 2);
    } else if (newIndex >= reordered.length - 1) {
      newPosition = (reordered[newIndex - 1]?.position ?? 0) + 1000;
    } else {
      const before = reordered[newIndex - 1];
      const after = reordered[newIndex + 1];
      newPosition = Math.floor(((before?.position ?? 0) + (after?.position ?? 0)) / 2);
    }

    const updatedBoards = reordered.map((b, i) =>
      b.id === moved.id ? { ...b, position: newPosition } : b
    );
    dispatch(optimisticReorderBoards(updatedBoards));
    dispatch(reorderBoard({ id: moved.id, data: { position: newPosition } }));
  };

  return (
    <Drawer variant="permanent" {...rest}>
      <Brand component={NavLink as any} to="/">
        <BrandIcon src={lightupLogo} alt="LightUp" />{" "}
        <Box ml={1}>
          LightUp <BrandChip label="PRO" />
        </Box>
      </Brand>
      <BoardList>
        <BoardSectionTitle variant="subtitle2">Boards</BoardSectionTitle>
        <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleBoardDragEnd} modifiers={[restrictToVerticalAxis, restrictToParentElement]}>
          <SortableContext items={boards.map((b) => b.id)} strategy={verticalListSortingStrategy}>
            <List disablePadding>
              {boards.map((board) => (
                <SortableBoardItem
                  key={board.id}
                  board={board}
                  isActive={board.id === activeBoardId}
                  onSelect={() => dispatch(setActiveBoard(board.id))}
                  onSettings={(selectedBoard) => setSettingsBoard(selectedBoard)}
                />
              ))}
            </List>
          </SortableContext>
        </DndContext>
        {addingBoard ? (
          <AddBoardRow>
            <AddBoardInput
              value={newBoardName}
              onChange={(e) => setNewBoardName(e.target.value)}
              placeholder="Board name..."
              autoFocus
              onKeyDown={(e) => {
                if (e.key === "Enter") handleAddBoard();
                if (e.key === "Escape") { setAddingBoard(false); setNewBoardName(""); }
              }}
              onBlur={() => {
                if (!newBoardName.trim()) {
                  setAddingBoard(false);
                  setNewBoardName("");
                }
              }}
            />
            <IconButton size="small" onClick={handleAddBoard} sx={{ color: "inherit", ml: 0.5 }}>
              <AddIcon fontSize="small" />
            </IconButton>
          </AddBoardRow>
        ) : (
          <AddBoardRow>
            <IconButton
              size="small"
              onClick={() => setAddingBoard(true)}
              sx={{ color: "rgba(255,255,255,0.5)" }}
            >
              <AddIcon fontSize="small" />
            </IconButton>
            <Typography
              variant="caption"
              sx={{ color: "rgba(255,255,255,0.4)", ml: 1, cursor: "pointer" }}
              onClick={() => setAddingBoard(true)}
            >
              New Board
            </Typography>
          </AddBoardRow>
        )}
      </BoardList>
      {settingsBoard && (
        <BoardSettingsDialog
          open={Boolean(settingsBoard)}
          boardId={settingsBoard.id}
          boardName={settingsBoard.name}
          onClose={() => setSettingsBoard(null)}
        />
      )}
      {!!showFooter && <Footer />}
    </Drawer>
  );
};

export default Sidebar;
