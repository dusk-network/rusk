type IconProp = {
  path: string;
  position?: "after" | "before";
  size?: IconSize;
};

type ButtonSize = "default" | "small";

type BadgeVariant = "neutral" | "success" | "warning" | "error";

type ButtonVariant = "primary" | "secondary" | "tertiary";

type CardGap = "small" | "default" | "medium" | "large";

type WizardButtonProps = {
  isAnchor?: boolean;
  href?: string;
  disabled?: boolean;
  icon?: IconProp | null;
  variant?: ButtonVariant;
  label?: string;
  action?: () => void;
};

type GapSize = "small" | "default" | "medium" | "large";

type IconSize = "small" | "default" | "large";

type GroupedSelectOptions = Record<string, SelectOption[] | string[]>;

type SelectOption = {
  disabled?: boolean;
  label?: string;
  value: string;
};

type StepperStep = {
  iconPath?: string;
  label: string;
};

type StepperVariant = "primary" | "secondary";

type SuspenseErrorVariant = "alert" | "details";

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
