import { Badge } from "@/components/ui/badge";

export function ProgramApplicationStatus({ status }: { status: string }) {
  switch (status) {
    case "open":
      return <Badge color="amber">Open</Badge>;
    case "approving":
      return <Badge color="blue">Approval in progress</Badge>;
    case "approved":
      return <Badge color="green">Approved</Badge>;
    case "closed":
      return <Badge>Closed</Badge>;
    default:
      return <Badge>{status}</Badge>;
  }
}
