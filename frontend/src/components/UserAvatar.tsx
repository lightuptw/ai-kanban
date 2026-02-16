import React from "react";
import { Avatar } from "@mui/material";
import { avatarUrl } from "../services/auth";

function hashToHue(str: string): number {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    hash = str.charCodeAt(i) + ((hash << 5) - hash);
  }
  return Math.abs(hash) % 360;
}

interface UserAvatarProps {
  userId?: string;
  nickname: string;
  hasAvatar?: boolean;
  size?: number;
}

const UserAvatar: React.FC<UserAvatarProps> = ({
  userId,
  nickname,
  hasAvatar,
  size = 32,
}) => {
  if (hasAvatar && userId) {
    return (
      <Avatar
        src={avatarUrl(userId)}
        sx={{ width: size, height: size }}
        alt={nickname}
      />
    );
  }

  const hue = hashToHue(userId || nickname);
  const initial = (nickname || "?").charAt(0).toUpperCase();

  return (
    <Avatar
      sx={{
        width: size,
        height: size,
        bgcolor: `hsl(${hue}, 65%, 50%)`,
        fontSize: size * 0.45,
      }}
      alt={nickname}
    >
      {initial}
    </Avatar>
  );
};

export default UserAvatar;
