import React, { useRef, useState } from "react";
import { useSelector, useDispatch } from "react-redux";
import styled from "@emotion/styled";
import { formatDistanceToNow } from "date-fns";

import {
  Badge,
  Box,
  Button,
  IconButton,
  List,
  ListItem,
  ListItemIcon,
  ListItemText,
  Popover as MuiPopover,
  SvgIcon,
  Tooltip,
  Typography,
} from "@mui/material";
import {
  Bell,
  AlertCircle,
  CheckCircle,
  ArrowRight,
  Eye,
  AlertTriangle,
} from "react-feather";

import type { RootState, AppDispatch } from "../../redux/store";
import type { NotificationType } from "../../types/kanban";
import {
  markRead,
  markAllRead,
  deleteNotification,
} from "../../store/slices/notificationSlice";

const Popover = styled(MuiPopover)`
  .MuiPaper-root {
    width: 360px;
    max-height: 480px;
    ${(props) => props.theme.shadows[1]};
    border: 1px solid ${(props) => props.theme.palette.divider};
  }
`;

const Indicator = styled(Badge)`
  .MuiBadge-badge {
    background: ${(props) => props.theme.header.indicator.background};
    color: ${(props) => props.theme.palette.common.white};
  }
`;

const NotificationHeader = styled(Box)`
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid ${(props) => props.theme.palette.divider};
  padding: ${(props) => props.theme.spacing(2)};
`;

const NotificationList = styled(List)`
  max-height: 360px;
  overflow-y: auto;
`;

const ICON_MAP: Record<NotificationType, React.ElementType> = {
  card_stage_changed: ArrowRight,
  ai_completed: CheckCircle,
  ai_question_pending: AlertCircle,
  review_requested: Eye,
  ai_error: AlertTriangle,
};

const COLOR_MAP: Record<NotificationType, string> = {
  card_stage_changed: "#1976d2",
  ai_completed: "#2e7d32",
  ai_question_pending: "#ed6c02",
  review_requested: "#9c27b0",
  ai_error: "#d32f2f",
};

function NavbarNotificationsDropdown() {
  const ref = useRef<HTMLButtonElement>(null);
  const [isOpen, setOpen] = useState(false);
  const dispatch = useDispatch<AppDispatch>();

  const { notifications, unreadCount } = useSelector(
    (state: RootState) => state.notifications
  );

  const handleOpen = () => setOpen(true);
  const handleClose = () => setOpen(false);

  const handleMarkAllRead = () => {
    dispatch(markAllRead());
  };

  const handleClickNotification = (id: string, isRead: boolean) => {
    if (!isRead) {
      dispatch(markRead(id));
    }
  };

  const handleDelete = (e: React.MouseEvent, id: string) => {
    e.stopPropagation();
    dispatch(deleteNotification(id));
  };

  return (
    <React.Fragment>
      <Tooltip title="Notifications">
        <IconButton color="inherit" ref={ref} onClick={handleOpen} size="large">
          <Indicator badgeContent={unreadCount > 0 ? unreadCount : undefined}>
            <Bell />
          </Indicator>
        </IconButton>
      </Tooltip>
      <Popover
        anchorOrigin={{ vertical: "bottom", horizontal: "center" }}
        anchorEl={ref.current}
        onClose={handleClose}
        open={isOpen}
      >
        <NotificationHeader>
          <Typography variant="subtitle1" color="textPrimary" fontWeight={600}>
            Notifications
            {unreadCount > 0 && ` (${unreadCount})`}
          </Typography>
          {unreadCount > 0 && (
            <Button size="small" onClick={handleMarkAllRead}>
              Mark all read
            </Button>
          )}
        </NotificationHeader>

        {notifications.length === 0 ? (
          <Box p={3} textAlign="center">
            <Typography variant="body2" color="textSecondary">
              No notifications
            </Typography>
          </Box>
        ) : (
          <NotificationList disablePadding>
            {notifications.map((n) => {
              const Icon = ICON_MAP[n.notification_type] || Bell;
              const color = COLOR_MAP[n.notification_type] || "#757575";

              return (
                <ListItem
                  key={n.id}
                  divider
                  onClick={() =>
                    handleClickNotification(n.id, n.is_read)
                  }
                  sx={{
                    cursor: "pointer",
                    backgroundColor: n.is_read
                      ? "transparent"
                      : "action.hover",
                    "&:hover": { backgroundColor: "action.selected" },
                  }}
                  secondaryAction={
                    <IconButton
                      edge="end"
                      size="small"
                      onClick={(e) => handleDelete(e, n.id)}
                      sx={{ opacity: 0.5, "&:hover": { opacity: 1 } }}
                    >
                      <Typography variant="caption" sx={{ fontSize: 14 }}>
                        ×
                      </Typography>
                    </IconButton>
                  }
                >
                  <ListItemIcon sx={{ minWidth: 36 }}>
                    <SvgIcon
                      fontSize="small"
                      sx={{ color }}
                    >
                      <Icon />
                    </SvgIcon>
                  </ListItemIcon>
                  <ListItemText
                    primary={n.title}
                    primaryTypographyProps={{
                      variant: "subtitle2",
                      color: "textPrimary",
                      fontWeight: n.is_read ? 400 : 600,
                    }}
                    secondary={
                      <>
                        {n.message}
                        <Typography
                          component="span"
                          variant="caption"
                          display="block"
                          color="textSecondary"
                          mt={0.5}
                        >
                          {formatDistanceToNow(new Date(n.created_at), {
                            addSuffix: true,
                          })}
                        </Typography>
                      </>
                    }
                  />
                </ListItem>
              );
            })}
          </NotificationList>
        )}
      </Popover>
    </React.Fragment>
  );
}

export default NavbarNotificationsDropdown;
