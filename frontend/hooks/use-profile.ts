import { useQuery, useQueries } from "@tanstack/react-query";
import { fetchProfile } from "@/lib/pubky/profiles";

const PROFILE_STALE_TIME = 5 * 60 * 1000; // 5 minutes

export function useProfile(pk: string | null) {
  return useQuery({
    queryKey: ["profile", pk],
    queryFn: () => fetchProfile(pk!),
    enabled: !!pk,
    staleTime: PROFILE_STALE_TIME,
  });
}

export function useProfileMap(pks: string[]): Map<string, string> {
  const queries = useQueries({
    queries: pks.map((pk) => ({
      queryKey: ["profile", pk],
      queryFn: () => fetchProfile(pk),
      staleTime: PROFILE_STALE_TIME,
    })),
  });

  const map = new Map<string, string>();
  pks.forEach((pk, i) => {
    const name = queries[i]?.data?.name;
    if (name) map.set(pk, name);
  });
  return map;
}
