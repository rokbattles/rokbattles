export function formatUTCShort(startTs?: number, endTs?: number): string | undefined {
  if (!startTs || startTs <= 0) return undefined;
  try {
    const startDate = new Date(startTs * 1000);
    const startMonth = String(startDate.getUTCMonth() + 1).padStart(2, "0");
    const startDay = String(startDate.getUTCDate()).padStart(2, "0");
    const startHour = String(startDate.getUTCHours()).padStart(2, "0");
    const startMinute = String(startDate.getUTCMinutes()).padStart(2, "0");

    if (!endTs || endTs <= 0) {
      return `UTC ${startMonth}/${startDay} ${startHour}:${startMinute}`;
    }

    const endDate = new Date(endTs * 1000);
    const endMonth = String(endDate.getUTCMonth() + 1).padStart(2, "0");
    const endDay = String(endDate.getUTCDate()).padStart(2, "0");
    const endHour = String(endDate.getUTCHours()).padStart(2, "0");
    const endMinute = String(endDate.getUTCMinutes()).padStart(2, "0");

    const sameDay =
      startDate.getUTCFullYear() === endDate.getUTCFullYear() &&
      startDate.getUTCMonth() === endDate.getUTCMonth() &&
      startDate.getUTCDate() === endDate.getUTCDate();

    if (sameDay) {
      return `UTC ${startMonth}/${startDay} ${startHour}:${startMinute} - ${endHour}:${endMinute}`;
    }

    return `UTC ${startMonth}/${startDay} ${startHour}:${startMinute} - ${endMonth}/${endDay} ${endHour}:${endMinute}`;
  } catch {
    return undefined;
  }
}
