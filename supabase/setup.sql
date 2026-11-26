-- Ascension System: database for cloud sync, parties and push reminders.
-- Run once in Supabase → SQL Editor → New query → paste → Run. Safe to re-run.

-- One row per user: the whole app state, plus a small digest the reminder job reads.
create table if not exists public.ascension_state (
  user_id    uuid primary key references auth.users (id) on delete cascade,
  state      jsonb  not null,
  updated_at bigint not null default 0,
  digest     jsonb
);
alter table public.ascension_state enable row level security;
drop policy if exists "own state" on public.ascension_state;
create policy "own state" on public.ascension_state
  for all to authenticated
  using (auth.uid() = user_id) with check (auth.uid() = user_id);

-- Parties: anyone signed in can look up a party by its code; members see each other's summary cards.
create table if not exists public.parties (
  code       text primary key check (code ~ '^[A-Z0-9]{4,8}$'),
  name       text not null check (char_length(name) <= 40),
  owner      uuid not null default auth.uid() references auth.users (id) on delete cascade,
  created_at timestamptz not null default now()
);
create table if not exists public.party_members (
  code       text not null references public.parties (code) on delete cascade,
  user_id    uuid not null default auth.uid() references auth.users (id) on delete cascade,
  name       text check (char_length(name) <= 40),
  card       jsonb,
  updated_at timestamptz not null default now(),
  primary key (code, user_id)
);
alter table public.parties enable row level security;
alter table public.party_members enable row level security;

create or replace function public.is_party_member(c text) returns boolean
  language sql stable security definer set search_path = public
  as $$ select exists (select 1 from public.party_members where code = c and user_id = auth.uid()) $$;

drop policy if exists "read parties" on public.parties;
create policy "read parties" on public.parties for select to authenticated using (true);
drop policy if exists "create party" on public.parties;
create policy "create party" on public.parties for insert to authenticated with check (owner = auth.uid());
drop policy if exists "owner deletes party" on public.parties;
create policy "owner deletes party" on public.parties for delete to authenticated using (owner = auth.uid());

drop policy if exists "members read" on public.party_members;
create policy "members read" on public.party_members for select to authenticated using (public.is_party_member(code));
drop policy if exists "join as self" on public.party_members;
create policy "join as self" on public.party_members for insert to authenticated with check (user_id = auth.uid());
drop policy if exists "update self" on public.party_members;
create policy "update self" on public.party_members for update to authenticated using (user_id = auth.uid()) with check (user_id = auth.uid());
drop policy if exists "leave as self" on public.party_members;
create policy "leave as self" on public.party_members for delete to authenticated using (user_id = auth.uid());

-- Push subscriptions: one row per device that turned reminders on.
create table if not exists public.push_subs (
  endpoint   text primary key,
  user_id    uuid not null references auth.users (id) on delete cascade,
  sub        jsonb not null,
  tz         text not null default 'UTC',
  sent       jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now()
);
alter table public.push_subs enable row level security;
drop policy if exists "own subs" on public.push_subs;
create policy "own subs" on public.push_subs
  for all to authenticated
  using (auth.uid() = user_id) with check (auth.uid() = user_id);

-- Let signed-in users reach these tables through the API (row-level security above still limits each row).
grant usage on schema public to authenticated;
grant select, insert, update, delete on public.ascension_state, public.parties, public.party_members, public.push_subs to authenticated;
grant execute on function public.is_party_member(text) to authenticated;
-- The hourly reminder job uses the service role.
grant usage on schema public to service_role;
grant select, update, delete on public.ascension_state, public.push_subs to service_role;

-- Live updates between devices and party members.
do $$
begin
  begin alter publication supabase_realtime add table public.ascension_state; exception when duplicate_object then null; end;
  begin alter publication supabase_realtime add table public.party_members; exception when duplicate_object then null; end;
end $$;
