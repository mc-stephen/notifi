"use client";

import { useSubscribers } from "@/hooks/use-subscribers";
import { PageHeader } from "@/components/dashboard/shared/page-header";
import { SubscriberSearch } from "@/components/dashboard/subscribers/subscriber-search";
import { SubscriberTable } from "@/components/dashboard/subscribers/subscriber-table";
import { EmptyState } from "@/components/dashboard/shared/empty-state";
import { Users } from "lucide-react";

export default function SubscribersPage() {
  const { subscribers, loading, search, setSearch } = useSubscribers();

  return (
    <div className="mx-auto w-full max-w-7xl space-y-4 p-5">
      <PageHeader
        title="Subscribers"
        description="Manage user records and device tokens."
      />

      <SubscriberSearch value={search} onChange={setSearch} />

      {!loading && subscribers.length === 0 ? (
        <EmptyState
          icon={<Users className="h-6 w-6" />}
          title="No subscribers found"
          description={
            search
              ? "Try a different search term."
              : "Subscribers will appear here once they register."
          }
        />
      ) : (
        <SubscriberTable subscribers={subscribers} loading={loading} />
      )}
    </div>
  );
}
