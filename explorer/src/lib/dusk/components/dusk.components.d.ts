type BadgeVariant = "neutral" | "success" | "warning" | "error" | "alt";

type BannerVariant = "info" | "warning" | "error";

type ButtonSize = "default" | "small";

type ButtonVariant = "primary" | "secondary" | "tertiary";

type GroupedSelectOptions = Record<string, SelectOption[] | string[]>;

type IconProp = {
  path: string;
  position?: "after" | "before";
  size?: IconSize;
};

type IconSize = "small" | "default" | "large";

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

type TextboxTypes =
  | "email"
  | "hidden"
  | "multiline"
  | "number"
  | "password"
  | "search"
  | "tel"
  | "text"
  | "url";

type TooltipType = "error" | "info" | "success" | "warning";

type ToastItem = {
  icon?: string;
  id: string;
  message: string;
  type: TooltipType;
};
