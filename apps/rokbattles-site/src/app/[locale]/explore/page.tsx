import { FunnelIcon, LanguagesIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Dropdown, DropdownButton, DropdownItem, DropdownMenu } from "@/components/ui/dropdown";
import { Heading } from "@/components/ui/heading";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

export default function Page() {
  return (
    <>
      <div className="flex items-end justify-between gap-4">
        <Heading>Explore Battle Reports</Heading>
        <div className="flex gap-x-2">
          <Dropdown>
            <DropdownButton plain aria-label="Change language">
              <LanguagesIcon data-slot="lucide" />
            </DropdownButton>
            <DropdownMenu anchor="bottom end">
              <DropdownItem href="/explore" locale="en">
                English
              </DropdownItem>
              <DropdownItem href="/explore" locale="de">
                Deutsch
              </DropdownItem>
              <DropdownItem href="/explore" locale="es">
                Español
              </DropdownItem>
              <DropdownItem href="/explore" locale="kr">
                한국어
              </DropdownItem>
            </DropdownMenu>
          </Dropdown>
          <Button className="-my-0.5">
            <FunnelIcon data-slot="lucide" />
            Filters
          </Button>
        </div>
      </div>
      <Table className="mt-8">
        <TableHead>
          <TableRow>
            <TableHeader>Time</TableHeader>
            <TableHeader>Participant 1 (Self)</TableHeader>
            <TableHeader>Participant 2 (Enemy)</TableHeader>
            <TableHeader>Total Battles</TableHeader>
          </TableRow>
        </TableHead>
        <TableBody>
          <TableRow>
            <TableCell className="tabular-nums text-zinc-500 dark:text-zinc-400 sm:w-1/6">
              UTC 9/20 15:04
            </TableCell>
            <TableCell className="sm:w-1/3">
              <div className="flex flex-col">
                <span>Ragnar Lodbrok</span>
                <span className="text-zinc-500 dark:text-zinc-400">Scipio Africanus</span>
              </div>
            </TableCell>
            <TableCell className="sm:w-1/3">
              <div className="flex flex-col">
                <span>Bai Qi</span>
                <span className="text-zinc-500 dark:text-zinc-400">William Wallace</span>
              </div>
            </TableCell>
            <TableCell className="tabular-nums  text-zinc-500 dark:text-zinc-400 sm:w-1/6">
              9
            </TableCell>
          </TableRow>
        </TableBody>
      </Table>
    </>
  );
}
