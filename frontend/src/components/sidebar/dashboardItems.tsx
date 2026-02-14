import { SidebarItemsType } from "../../types/sidebar";

import { Layout } from "react-feather";

const pagesSection = [
  {
    href: "/",
    icon: Layout,
    title: "Board",
  },
] as SidebarItemsType[];

const navItems = [
  {
    title: "",
    pages: pagesSection,
  },
];

export default navItems;
