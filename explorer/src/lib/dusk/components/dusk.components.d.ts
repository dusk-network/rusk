type BadgeVariant = "neutral" | "success" | "warning" | "error";

type GroupedSelectOptions = Record<string, SelectOption[] | string[]>;

type IconProp = {
  path: string;
  position?: "after" | "before";
  size?: IconSize;
};

type IconSize = "small" | "normal" | "large";

type NavListProp = NavListItem[];

type NavListItem = {
  title: string;
  link: string;
};

type SelectOption = {
  disabled?: boolean;
  label?: string;
  value: string;
};

type TooltipType = "error" | "info" | "success" | "warning";
