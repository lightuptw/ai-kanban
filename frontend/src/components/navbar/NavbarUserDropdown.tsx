import React from "react";
import styled from "@emotion/styled";
import { Power } from "react-feather";

import {
  Tooltip,
  IconButton as MuiIconButton,
} from "@mui/material";

const IconButton = styled(MuiIconButton)`
  svg {
    width: 22px;
    height: 22px;
  }
`;

function NavbarUserDropdown() {
  return (
    <React.Fragment>
      <Tooltip title="Account">
        <IconButton color="inherit" size="large">
          <Power />
        </IconButton>
      </Tooltip>
    </React.Fragment>
  );
}

export default NavbarUserDropdown;
