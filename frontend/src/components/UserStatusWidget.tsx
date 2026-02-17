import React from "react";
import styled from "@emotion/styled";
import { Avatar, Badge, Typography } from "@mui/material";
import { API_BASE_URL } from "../constants";
import { useAuth } from "../hooks/useAuth";

type UserStatusWidgetProps = {
  onClick?: () => void;
};

const Widget = styled.button`
  position: fixed;
  right: 24px;
  bottom: 24px;
  z-index: 1200;
  display: inline-flex;
  align-items: center;
  gap: ${(props) => props.theme.spacing(1.25)};
  padding: 8px 16px;
  border: 0;
  border-radius: 24px;
  background: rgba(0, 0, 0, 0.72);
  color: rgba(255, 255, 255, 0.96);
  cursor: pointer;
  transition: box-shadow 180ms ease, transform 180ms ease;

  &:hover {
    box-shadow: 0 12px 24px rgba(0, 0, 0, 0.35);
    transform: translateY(-1px);
  }
`;

const UserBadge = styled(Badge)`
  .MuiBadge-badge {
    background-color: #29bf5b;
    box-shadow: 0 0 0 2px rgba(0, 0, 0, 0.72);
  }
`;

const UserAvatar = styled(Avatar)`
  width: 40px;
  height: 40px;
`;

const UserName = styled(Typography)`
  color: inherit;
  font-weight: 600;
`;

const UserStatusWidget: React.FC<UserStatusWidgetProps> = ({ onClick }) => {
  const { user } = useAuth();
  const avatarSrc = user?.avatar_url ? `${API_BASE_URL}${user.avatar_url}` : undefined;
  const fallbackName = user?.nickname || "User";
  const initial = fallbackName.trim().charAt(0).toUpperCase() || "U";

  return (
    <Widget type="button" onClick={onClick} aria-label="Open profile">
      <UserBadge
        overlap="circular"
        anchorOrigin={{ vertical: "bottom", horizontal: "right" }}
        variant="dot"
      >
        <UserAvatar src={avatarSrc}>{initial}</UserAvatar>
      </UserBadge>
      <UserName variant="body2">{fallbackName}</UserName>
    </Widget>
  );
};

export default UserStatusWidget;
