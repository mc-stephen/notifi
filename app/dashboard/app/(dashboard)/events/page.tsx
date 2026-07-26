"use client";

import { useState, useMemo } from "react";
import { useEvents } from "@/hooks";
import { PageHeader } from "@/components/custom/page-header";
import { JsonViewer } from "@/components/custom/json-viewer";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { format } from "date-fns";
import {
  Search,
  Clock,
  X,
} from "lucide-react";

const EVENT_TYPE_COLORS: Record<string, string> = {
  queued: "bg-info/15 text-info",
  worker_assigned: "bg-muted text-muted-foreground",
  provider_selected: "bg-muted text-muted-foreground",
  sent: "bg-info/15 text-info",
  delivered: "bg-success/15 text-success",
  opened: "bg-success/15 text-success",
  clicked: "bg-success/15 text-success",
  failed: "bg-destructive/15 text-destructive",
  retried: "bg-warning/15 text-warning",
  cancelled: "bg-muted text-muted-foreground",
};

export default function EventsPage() {
  const { items: allEvents } = useEvents(1, 200);
  const [search, setSearch] = useState("");
  const [selectedType, setSelectedType] = useState<string | null>(null);
  const [selectedEvent, setSelectedEvent] = useState<(typeof allEvents)[number] | null>(null);

  const eventTypes = useMemo(() => [...new Set(allEvents.map((e) => e.type))], [allEvents]);

  const filteredEvents = useMemo(() => {
    let filtered = [...allEvents];
    if (search) {
      const q = search.toLowerCase();
      filtered = filtered.filter(
        (e) =>
          e.id.toLowerCase().includes(q) ||
          e.notificationId.toLowerCase().includes(q) ||
          e.type.toLowerCase().includes(q),
      );
    }
    if (selectedType) {
      filtered = filtered.filter((e) => e.type === selectedType);
    }
    return filtered;
  }, [allEvents, search, selectedType]);

  const typeCounts = useMemo(() => {
    const counts: Record<string, number> = {};
    allEvents.forEach((e) => {
      counts[e.type] = (counts[e.type] || 0) + 1;
    });
    return counts;
  }, [allEvents]);

  return (
    <div className="space-y-6">
      <PageHeader
        title="Events"
        description={`${allEvents.length} events in the last 24 hours`}
        breadcrumbs={[{ label: "Dashboard", href: "/" }, { label: "Events" }]}
        actions={
          <Button size="sm" variant="outline" className="gap-1.5">
            <Clock className="size-3.5" /> Real-time
          </Button>
        }
      />

      {/* Event type summary */}
      <div className="grid gap-3 md:grid-cols-5">
        {eventTypes.slice(0, 5).map((type) => (
          <button
            key={type}
            onClick={() => setSelectedType(selectedType === type ? null : type)}
            className={`flex items-center justify-between rounded-lg border p-3 text-left transition-colors ${
              selectedType === type ? "border-primary bg-primary/5" : "hover:bg-muted/50"
            }`}
          >
            <div className="flex items-center gap-2">
              <div className={`size-2 rounded-full ${EVENT_TYPE_COLORS[type]?.split(" ")[0] ?? "bg-muted"}`} />
              <span className="text-sm font-medium capitalize">{type.replace(/_/g, " ")}</span>
            </div>
            <Badge variant="secondary" className="text-xs">{typeCounts[type] ?? 0}</Badge>
          </button>
        ))}
      </div>

      {/* Search */}
      <div className="flex items-center gap-3">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground" />
          <Input
            placeholder="Search by ID, notification, or type..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="pl-8 h-8 text-sm"
          />
        </div>
        {selectedType && (
          <Button variant="ghost" size="sm" onClick={() => setSelectedType(null)} className="gap-1">
            <X className="size-3" /> Clear filter
          </Button>
        )}
      </div>

      {/* Events table */}
      <Card>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Event</TableHead>
                <TableHead>Type</TableHead>
                <TableHead>Notification</TableHead>
                <TableHead>Provider</TableHead>
                <TableHead>Timestamp</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {filteredEvents.slice(0, 50).map((event) => (
                <TableRow
                  key={event.id}
                  className="cursor-pointer hover:bg-muted/30"
                  onClick={() => setSelectedEvent(event)}
                >
                  <TableCell>
                    <span className="font-mono text-xs">{event.id}</span>
                  </TableCell>
                  <TableCell>
                    <Badge variant="secondary" className={`text-xs ${EVENT_TYPE_COLORS[event.type] ?? ""}`}>
                      {event.type.replace(/_/g, " ")}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    <span className="font-mono text-xs">{event.notificationId}</span>
                  </TableCell>
                  <TableCell>
                    <span className="text-xs text-muted-foreground">{event.provider ?? "—"}</span>
                  </TableCell>
                  <TableCell>
                    <span className="text-xs text-muted-foreground whitespace-nowrap">
                      {format(new Date(event.timestamp), "MMM d, HH:mm:ss")}
                    </span>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      {/* Event detail dialog */}
      <Dialog open={!!selectedEvent} onOpenChange={() => setSelectedEvent(null)}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              Event Details
              {selectedEvent && (
                <Badge variant="secondary" className={`text-xs ${EVENT_TYPE_COLORS[selectedEvent.type] ?? ""}`}>
                  {selectedEvent.type.replace(/_/g, " ")}
                </Badge>
              )}
            </DialogTitle>
          </DialogHeader>
          {selectedEvent && (
            <div className="space-y-4">
              <div className="grid grid-cols-2 gap-4 text-sm">
                <div>
                  <span className="text-muted-foreground">Event ID</span>
                  <p className="font-mono text-xs mt-1">{selectedEvent.id}</p>
                </div>
                <div>
                  <span className="text-muted-foreground">Notification</span>
                  <p className="font-mono text-xs mt-1">{selectedEvent.notificationId}</p>
                </div>
                <div>
                  <span className="text-muted-foreground">Provider</span>
                  <p className="text-xs mt-1">{selectedEvent.provider ?? "—"}</p>
                </div>
                <div>
                  <span className="text-muted-foreground">Request ID</span>
                  <p className="font-mono text-xs mt-1">{selectedEvent.requestId ?? "—"}</p>
                </div>
                <div className="col-span-2">
                  <span className="text-muted-foreground">Timestamp</span>
                  <p className="text-xs mt-1">{format(new Date(selectedEvent.timestamp), "MMM d, yyyy HH:mm:ss")}</p>
                </div>
              </div>
              {selectedEvent.metadata && Object.keys(selectedEvent.metadata).length > 0 && (
                <div>
                  <span className="text-sm text-muted-foreground">Metadata</span>
                  <div className="mt-2">
                    <JsonViewer data={selectedEvent.metadata} />
                  </div>
                </div>
              )}
              {selectedEvent.diagnosticInfo && Object.keys(selectedEvent.diagnosticInfo).length > 0 && (
                <div>
                  <span className="text-sm text-muted-foreground">Diagnostic Info</span>
                  <div className="mt-2">
                    <JsonViewer data={selectedEvent.diagnosticInfo} />
                  </div>
                </div>
              )}
            </div>
          )}
        </DialogContent>
      </Dialog>
    </div>
  );
}
