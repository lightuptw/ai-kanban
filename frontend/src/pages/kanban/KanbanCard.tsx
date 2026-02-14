import React from "react";
import styled from "@emotion/styled";
import { MessageCircle } from "react-feather";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import {
  Avatar,
  AvatarGroup as MuiAvatarGroup,
  Typography as MuiTypography,
} from "@mui/material";
import { spacing } from "@mui/system";

const TaskWrapper = styled.div<{ isDragging?: boolean }>`
  border: 1px solid ${(props) => props.theme.palette.grey[300]};
  border-radius: 7px;
  margin: 2px 2px;
  padding: ${(props) => props.theme.spacing(2)} ${(props) => props.theme.spacing(3)};
  cursor: pointer;
  opacity: ${(props) => (props.isDragging ? 0.5 : 1)};
  transform: ${(props) => (props.isDragging ? "scale(1.05)" : "scale(1)")};
  box-shadow: ${(props) =>
    props.isDragging
      ? "rgba(50, 50, 93, 0.25) 0px 13px 27px -5px, rgba(0, 0, 0, 0.3) 0px 8px 16px -8px"
      : "none"};
  transition: transform 0.2s ease, box-shadow 0.2s ease, background-color 0.2s ease;
  background: ${(props) => props.theme.palette.background.paper};
  position: relative;

  &:hover {
    background: skyblue;
  }
`;

const AvatarGroup = styled(MuiAvatarGroup)`
  display: inline-flex;
`;

const TaskAvatars = styled.div`
  margin-top: ${(props) => props.theme.spacing(1)};
`;

const MessageCircleIcon = styled(MessageCircle)`
  color: ${(props) => props.theme.palette.grey[500]};
  vertical-align: middle;
`;

const TaskBadge = styled.div`
  background: ${(props) => props.color};
  width: 40px;
  height: 6px;
  border-radius: 6px;
  display: inline-block;
  margin-right: ${(props) => props.theme.spacing(2)};
`;

const TaskNotifications = styled.div`
  display: flex;
  position: absolute;
  bottom: ${(props) => props.theme.spacing(2)};
  right: ${(props) => props.theme.spacing(3)};
`;

const TaskNotificationsAmount = styled.div`
  color: ${(props) => props.theme.palette.grey[500]};
  font-weight: 600;
  margin-right: ${(props) => props.theme.spacing(1)};
  line-height: 1.75;
`;

const Typography = styled(MuiTypography)(spacing);

const TaskTitle = styled(Typography)`
  font-weight: 600;
  font-size: 15px;
  margin-right: ${(props) => props.theme.spacing(10)};
`;

interface KanbanCardProps {
  id: string;
  title: string;
  badges?: string[];
  notifications?: number;
  avatars?: number[];
  onClick?: () => void;
}

export const KanbanCard: React.FC<KanbanCardProps> = ({
  id,
  title,
  badges = [],
  notifications = 0,
  avatars = [],
  onClick,
}) => {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id,
  });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  const handleClick = (e: React.MouseEvent) => {
    console.log('Card clicked!');
    if (onClick) {
      onClick();
    }
  };

  return (
    <div ref={setNodeRef} style={style} {...attributes} {...listeners}>
      <TaskWrapper isDragging={isDragging} onClick={handleClick}>
        {badges.map((color, i) => (
          <TaskBadge color={color} key={i} />
        ))}

        <TaskTitle variant="body1" gutterBottom>
          {title}
        </TaskTitle>

        <TaskAvatars>
          <AvatarGroup max={3}>
            {avatars.map((avatar, i) => (
              <Avatar
                src={`/static/img/avatars/avatar-${avatar}.jpg`}
                key={i}
              />
            ))}
          </AvatarGroup>
        </TaskAvatars>

        {notifications > 0 && (
          <TaskNotifications>
            <TaskNotificationsAmount>{notifications}</TaskNotificationsAmount>
            <MessageCircleIcon />
          </TaskNotifications>
        )}
      </TaskWrapper>
    </div>
  );
};
