# AST Research: DeepSearch Sub-Agent Builder

## DeepSearch tool builder (codelet/tools/src/)
- Sub-agent is built with a set of read-only tools
- Default tools: Read, Grep, AstGrep, Glob, Ls, Bash, SessionSearch (7 tools)
- Tool set is constructed in the DeepSearch handler before spawning

## graph::is_graph_initialized()
- Returns bool — fast check for graph DB singleton availability
- From codelet/napi/src/graph/mod.rs

## Integration point
- Conditional tool addition: `if graph::is_graph_initialized() { tools.push(graph_search_tool); }`
- System prompt enrichment: query graph for related concepts before spawning
