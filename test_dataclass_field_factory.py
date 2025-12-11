from dataclasses import dataclass, field
from typing import Set

@dataclass
class State:
    executed_events: Set[str] = field(default_factory=set)
