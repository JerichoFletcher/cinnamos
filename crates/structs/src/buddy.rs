use core::{alloc::Layout, ptr::NonNull, u8};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuddyTreeNode {
    Free,
    Used,
    Split { max_free_order: u8 },
}

#[derive(Debug, Clone, Copy)]
pub struct BuddyTreeAllocation {
    start: usize,
    count: usize,
}

impl BuddyTreeAllocation {
    pub const fn start(&self) -> usize {
        self.start
    }

    pub const fn count(&self) -> usize {
        self.count
    }

    pub const fn end(&self) -> usize {
        self.start + self.count
    }
}

pub struct BuddyTreeAllocator {
    max_order: u8,
    nodes: [BuddyTreeNode],
}

impl BuddyTreeAllocator {
    pub fn layout(max_order: u8) -> Option<Layout> {
        let layout = Layout::new::<u8>();
        let node_count: usize = (1 << (max_order + 1)) - 1;
        let (layout, _) = layout
            .extend(Layout::new::<BuddyTreeNode>().repeat(node_count).ok()?.0)
            .ok()?;
        Some(layout.pad_to_align())
    }

    /// # Safety
    /// `at` must point to a memory region that respects [BuddyTreeAllocator::layout](BuddyTreeAllocator::layout).
    pub unsafe fn create(at: *mut (), max_order: u8) -> NonNull<Self> {
        let node_count: usize = (1 << (max_order + 1)) - 1;
        let ptr: *mut Self = core::ptr::from_raw_parts_mut(at, node_count);

        unsafe {
            (&raw mut (*ptr).max_order).write(max_order);
        }
        for i in 0..node_count {
            unsafe {
                (&raw mut (*ptr).nodes[i]).write(BuddyTreeNode::Free);
            }
        }
        unsafe { NonNull::new_unchecked(ptr) }
    }

    pub fn alloc(&mut self, count: usize) -> Option<BuddyTreeAllocation> {
        let (eff_count, eff_order) = Self::get_effective_count(count);
        if eff_order > self.max_order {
            return None;
        }

        let mut i: usize = 0;
        for order in ((eff_order + 1)..=self.max_order).rev() {
            match self.nodes[i] {
                BuddyTreeNode::Used => return None,
                BuddyTreeNode::Free => {
                    self.nodes[i] = BuddyTreeNode::Split {
                        max_free_order: order - 1,
                    };

                    let (l, r) = Self::children_of(i);
                    self.nodes[l] = BuddyTreeNode::Free;
                    self.nodes[r] = BuddyTreeNode::Free;
                    i = l;
                }
                BuddyTreeNode::Split { max_free_order } => {
                    if eff_order > max_free_order {
                        return None;
                    }
                    let (l, r) = Self::children_of(i);

                    if let BuddyTreeNode::Free = self.nodes[l] {
                        i = l;
                    } else if let BuddyTreeNode::Free = self.nodes[r] {
                        i = r;
                    } else if let BuddyTreeNode::Split { max_free_order } = self.nodes[l]
                        && max_free_order != u8::MAX
                        && eff_order < max_free_order
                    {
                        i = l;
                    } else if let BuddyTreeNode::Split { max_free_order } = self.nodes[r]
                        && max_free_order != u8::MAX
                        && eff_order < max_free_order
                    {
                        i = r;
                    } else {
                        return None;
                    }
                }
            }
        }

        self.nodes[i] = BuddyTreeNode::Used;
        let a = BuddyTreeAllocation {
            start: i,
            count: eff_count,
        };

        i = Self::parent_of(i);
        for order in (eff_order + 1)..=self.max_order {
            if let BuddyTreeNode::Split { max_free_order: _ } = self.nodes[i] {
                let (l, r) = Self::children_of(i);
                if let BuddyTreeNode::Used = self.nodes[l]
                    && let BuddyTreeNode::Used = self.nodes[r]
                {
                    self.nodes[i] = BuddyTreeNode::Split {
                        max_free_order: u8::MAX,
                    };
                } else if let BuddyTreeNode::Free = self.nodes[l] {
                    self.nodes[i] = BuddyTreeNode::Split {
                        max_free_order: order,
                    };
                } else if let BuddyTreeNode::Free = self.nodes[r] {
                    self.nodes[i] = BuddyTreeNode::Split {
                        max_free_order: order,
                    };
                } else if let BuddyTreeNode::Split {
                    max_free_order: l_ord,
                } = self.nodes[l]
                    && let BuddyTreeNode::Split {
                        max_free_order: r_ord,
                    } = self.nodes[r]
                {
                    let ord = if l_ord == u8::MAX {
                        r_ord
                    } else if r_ord == u8::MAX {
                        l_ord
                    } else {
                        l_ord.max(r_ord)
                    };
                    self.nodes[i] = BuddyTreeNode::Split {
                        max_free_order: ord,
                    };
                }
            }
        }
        Some(a)
    }

    pub const fn total_page_count(&self) -> usize {
        1 << self.max_order
    }

    const fn get_effective_count(count: usize) -> (usize, u8) {
        let eff_count = count.next_power_of_two();
        let eff_order = eff_count.trailing_zeros() as u8;
        (eff_count, eff_order)
    }

    #[inline]
    const fn children_of(index: usize) -> (usize, usize) {
        (2 * index + 1, 2 * index + 2)
    }

    #[inline]
    const fn parent_of(index: usize) -> usize {
        index / 2
    }
}
