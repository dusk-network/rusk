type IconProp = {
  path: string;
  position?: "after" | "before";
  size?: IconSize;
};

type ButtonSize = "normal" | "small";

type BadgeVariant = "neutral" | "success" | "warning" | "error";

/**
 * Removed the "primary" variant for now.
 * Will be reinstated later when we formalize our language.
 */
type ButtonVariant = "secondary" | "tertiary" | "quaternary";

type WizardButtonProps = {
  isAnchor?: boolean;
  href?: string;
  disabled?: boolean;
  icon?: IconProp | null;
  variant?: ButtonVariant;
  label?: string;
  action?: () => void;
};

type GapSize = "small" | "normal" | "large";

type IconSize = "small" | "normal" | "large";

type GroupedSelectOptions = Record<string, SelectOption[] | string[]>;

type SelectOption = {
  disabled?: boolean;
  label?: string;
  value: string;
};

type SuspenceErrorVariant = "alert" | "details";

type TabItem = {
  icon?: IconProp;
  id: string;
  label?: string;
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

type MnemonicType = "authenticate" | "validate";

type ToastItem = {
  icon?: string;
  id: string;
  message: string;
  type: TooltipType;
};
