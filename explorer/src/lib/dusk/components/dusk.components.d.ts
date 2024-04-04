type GroupedSelectOptions = Record<string, SelectOption[] | string[]>;

type SelectOption = {
  disabled?: boolean;
  label?: string;
  value: string;
};

type NavListProp = NavListItem[];

type NavListItem = {
  title: string;
  link: string;
};
