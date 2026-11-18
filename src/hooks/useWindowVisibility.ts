import { useEffect, useState } from "react";
import { subscribeWindowVisibility } from "../services/windowVisibility";

export function useWindowVisibility() {
  const [visible, setVisible] = useState(!document.hidden);
  useEffect(() => subscribeWindowVisibility(setVisible), []);
  return visible;
}
