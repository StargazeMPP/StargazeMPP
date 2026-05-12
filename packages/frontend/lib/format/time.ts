import { formatDistanceToNowStrict, format } from "date-fns";

export function relativeTime(date: Date | number): string {
  return formatDistanceToNowStrict(date, { addSuffix: true });
}

export function clockTime(date: Date | number = new Date()): string {
  return format(date, "HH:mm");
}

export function isoDay(date: Date | number): string {
  return format(date, "yyyy-MM-dd");
}
