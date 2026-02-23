-- Add strategy-advisor agent role used by the AI engine OODA loop.
--
-- The AI engine creates agent tasks with role_name = 'strategy-advisor'
-- for competitive intelligence gathering (see ai_engine.rs RequestAgentIntel).

INSERT INTO agent_roles (name, description, domains, default_permission_level, default_model, system_prompt, default_max_cost_usd) VALUES
  ('strategy-advisor',
   'AI engine strategy advisor — analyzes competitive landscape, suggests search form priorities',
   '["engine", "research"]',
   0,
   'haiku',
   'You are a strategy advisor for darkreach. Analyze competitive landscape data, prime discovery rates, and search form efficiency to recommend optimal resource allocation.',
   1.00)
ON CONFLICT (name) DO NOTHING;
