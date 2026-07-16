import React from "react";

type Props = any;
const node = (tag: keyof React.JSX.IntrinsicElements, props: Props) => {
  const { children, component, endIcon, noWrap, startIcon, ...rest } = props;
  delete rest.sx;
  return React.createElement(component || tag, rest, children);
};
export const Box = (p: Props) => node("div", p);
export const Stack = (p: Props) => node("div", p);
export const Typography = (p: Props) =>
  node(p.variant?.startsWith("h") ? p.variant : "p", p);
export const Button = (p: Props) => node("button", p);
export const Card = (p: Props) => node("section", p);
export const CardContent = (p: Props) => node("div", p);
export const Paper = (p: Props) => node("section", p);
export const Chip = (p: Props) =>
  node("span", { ...p, children: p.label ?? p.children });
export const Alert = (p: Props) => node("div", p);
export const Divider = (p: Props) => node("hr", p);
export const CircularProgress = (p: Props) => (
  <span
    className="inline-block size-5 animate-spin rounded-full border-2 border-current border-t-transparent"
    {...p}
  />
);
export const LinearProgress = (p: Props) => (
  <div className="h-2 w-full overflow-hidden rounded-full bg-muted">
    <div className="h-full bg-primary" style={{ width: `${p.value ?? 0}%` }} />
  </div>
);
export const TextField = (p: Props) => <input {...p} value={p.value ?? ""} />;
export const Dialog = (p: Props) =>
  p.open ? (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
      <div className="max-h-[90vh] w-full max-w-2xl overflow-auto rounded-xl border border-border bg-background p-6">
        {p.children}
      </div>
    </div>
  ) : null;
export const DialogTitle = (p: Props) => node("h2", p);
export const DialogContent = (p: Props) => node("div", p);
export const DialogContentText = (p: Props) => node("p", p);
export const DialogActions = (p: Props) => node("div", p);
export const Drawer = (p: Props) =>
  p.open ? (
    <div className="fixed inset-0 z-50 flex justify-end bg-black/50">
      <aside className="h-full w-full max-w-md overflow-auto bg-background p-6">
        {p.children}
      </aside>
    </div>
  ) : null;
export const IconButton = (p: Props) => node("button", p);
export const Tooltip = (p: Props) => node("span", p);
export const Checkbox = (p: Props) => <input type="checkbox" {...p} />;
export const FormGroup = (p: Props) => node("div", p);
export const FormControlLabel = (p: Props) => (
  <label className="inline-flex items-center gap-2">
    {p.control}
    {p.label}
  </label>
);
export const Tabs = (p: Props) => node("div", p);
export const Tab = (p: Props) => (
  <button {...p}>{p.label ?? p.children}</button>
);
export const ToggleButton = (p: Props) => node("button", p);
export const ToggleButtonGroup = (p: Props) => node("div", p);
export const Select = (p: Props) => <select {...p}>{p.children}</select>;
export const MenuItem = (p: Props) => <option {...p}>{p.children}</option>;
export const FormControl = (p: Props) => node("label", p);
export const FormLabel = (p: Props) => node("span", p);
export const RadioGroup = (p: Props) => node("div", p);
export const Radio = (p: Props) => <input type="radio" {...p} />;
export const Table = (p: Props) => node("table", p);
export const TableBody = (p: Props) => node("tbody", p);
export const TableCell = (p: Props) => node("td", p);
export const TableContainer = (p: Props) => node("div", p);
export const TableHead = (p: Props) => node("thead", p);
export const TableRow = (p: Props) => node("tr", p);
export const TablePagination = (p: Props) => node("div", p);
export const Avatar = (p: Props) => node("span", p);
export const Menu = (p: Props) => (p.open ? node("div", p) : null);
export const MenuItemText = (p: Props) => node("span", p);
export const ListItemIcon = (p: Props) => node("span", p);
export const ListItemText = (p: Props) => node("span", p);
export const Pagination = (p: Props) => node("div", p);
export const Skeleton = (p: Props) => (
  <span
    className="inline-block h-4 w-full animate-pulse rounded bg-muted"
    {...p}
  />
);
export const Autocomplete = (p: Props) => node("div", p);
export const InputAdornment = (p: Props) => node("span", p);
export const Grid = (p: Props) => node("div", p);

const Icon = (p: Props) => (
  <span aria-hidden="true" {...p}>
    ◦
  </span>
);
export const SmartToyOutlinedIcon = Icon;
export const EditOutlinedIcon = Icon;
export const DeleteOutlineIcon = Icon;
export const ContentCopyOutlinedIcon = Icon;
export const FileDownloadOutlinedIcon = Icon;
export const WarningAmberOutlinedIcon = Icon;
export const FlagOutlinedIcon = Icon;
export const TagOutlinedIcon = Icon;
export const AddOutlinedIcon = Icon;
export const ArrowForwardOutlinedIcon = Icon;
export const AutoAwesomeOutlinedIcon = Icon;
export const CheckOutlinedIcon = Icon;
export const ArrowBackOutlinedIcon = Icon;
export const ExitToAppOutlinedIcon = Icon;
export const Flag = Icon;
export const FlagIcon = Icon;
export const ArrowBack = Icon;
export const ArrowBackIcon = Icon;
export const CancelOutlinedIcon = Icon;
export const CheckCircle = Icon;
export const CheckCircleOutlineIcon = Icon;
export const Close = Icon;
export const CloseIcon = Icon;
