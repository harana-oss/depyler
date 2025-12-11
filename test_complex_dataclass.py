from dataclasses import dataclass, field
from typing import Set, List, Dict

@dataclass
class ComplexState:
    # Test different default_factory types
    events: Set[str] = field(default_factory=set)
    items: List[int] = field(default_factory=list)
    metadata: Dict[str, int] = field(default_factory=dict)
    
    # Test field(default=value)
    count: int = field(default=0)
    name: str = field(default="unknown")
    
    # Test regular default (not using field())
    enabled: bool = True
    
    # Test no default
    id: int
