type BannerVariant = "info" | "warning" | "error";

type DashboardNavItem = {
  href: string;
  icons?: DashboardNavItemIconProp[];
  id: string;
  label: string;
};

type DashboardNavItemIconProp = {
  path: string;
};
