# AgolTui

An ArcGIS Online admin cli tool written in rust. UI/widgets created using Ratatui. Main goal of this project is to simplify the process of identifying broken data connections in ArcGIS Online Web Maps/Apps when source data needs to be republished. 
<img width="1261" height="1390" alt="screenshot-2026-05-01_11-49-11" src="https://github.com/user-attachments/assets/673f6fba-051d-46ee-b43c-ff3aede593d5" />

## Features

- List all ArcGIS Online content within your organization
- List all references per ArcGIS Online item
- List content per user
- Identify source data items that have zero references
- Search by Keyword, Username/Email, and Item Id
- Vim navigation controls

## Instructions


Users will need ArcGIS Online OAuth 2.0 app credentials with General & Admin View privileges  (Members, Groups, Content). 
Users will need to set the following env vars:
- ORG_WIDE_SEARCH_AND_CATALOG_CLIENT_ID
- ORG_WIDE_SEARCH_AND_CATALOG_CLIENT_SECRET

Organization ID will be extracted during the Oauth 2.0 token flow and used in org wide queries.

## Keybinds
- j/Down Key - Traverse list down
- k/Up Key - Traverse list up
- 0 - Zero references filter
- s - Search widget
  - F1 - toggle Keyword search
  - F2 - toggle Email/Username search
  - F3 - toggle Item Id search
- u - List item totals by User widget
- q - Quit App
