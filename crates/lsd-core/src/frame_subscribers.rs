/// The original clock's newest-first list and live notification cursor.
///
/// The cursor advances before each callback. Removing the next subscriber
/// advances it again, while a new head waits until the next notification.
#[derive(Debug, Clone)]
pub struct FrameSubscribers<T> {
    nodes: Vec<Option<Node<T>>>,
    free: Vec<usize>,
    head: Option<usize>,
    cursor: Option<usize>,
    len: usize,
}

#[derive(Debug, Clone, Copy)]
struct Node<T> {
    value: T,
    next: Option<usize>,
}

impl<T> Default for FrameSubscribers<T> {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            free: Vec::new(),
            head: None,
            cursor: None,
            len: 0,
        }
    }
}

impl<T: Copy + PartialEq> FrameSubscribers<T> {
    /// Prepend a subscription. Like the original base list, duplicates are allowed.
    pub fn subscribe(&mut self, value: T) {
        let node = Some(Node {
            value,
            next: self.head,
        });
        let index = if let Some(index) = self.free.pop() {
            self.nodes[index] = node;
            index
        } else {
            self.nodes.push(node);
            self.nodes.len() - 1
        };
        self.head = Some(index);
        self.len += 1;
    }

    /// Match frame_phase_remove_parent, then remove the first matching list node.
    pub fn remove(&mut self, value: T) -> bool {
        if let Some(index) = self.cursor
            && self.nodes[index].as_ref().unwrap().value == value
        {
            self.cursor = self.nodes[index].as_ref().unwrap().next;
        }
        let mut current = self.head;
        let mut previous: Option<usize> = None;
        while let Some(index) = current {
            let node = self.nodes[index].as_ref().unwrap();
            if node.value == value {
                let next = node.next;
                if let Some(previous) = previous {
                    self.nodes[previous].as_mut().unwrap().next = next;
                } else {
                    self.head = next;
                }
                self.nodes[index] = None;
                self.free.push(index);
                self.len -= 1;
                return true;
            }
            previous = current;
            current = node.next;
        }
        false
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn contains(&self, value: T) -> bool {
        self.order().contains(&value)
    }

    pub fn order(&self) -> Vec<T> {
        let mut result = Vec::with_capacity(self.len);
        let mut current = self.head;
        while let Some(index) = current {
            let node = self.nodes[index].as_ref().unwrap();
            result.push(node.value);
            current = node.next;
        }
        result
    }

    pub fn retain(&mut self, mut keep: impl FnMut(T) -> bool) {
        for value in self.order() {
            if !keep(value) {
                self.remove(value);
            }
        }
    }

    /// Clear a retired scene's subscriptions and cancel any pending traversal.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn begin_notification(&mut self) {
        self.cursor = self.head;
    }

    pub fn next_subscriber(&mut self) -> Option<T> {
        let node = self.nodes[self.cursor?].as_ref().unwrap();
        self.cursor = node.next;
        Some(node.value)
    }

    /// Also represents the original frame reset's cancellation of its cursor.
    pub fn end_notification(&mut self) {
        self.cursor = None;
    }

    pub fn next_pending(&self) -> Option<T> {
        Some(self.nodes[self.cursor?].as_ref().unwrap().value)
    }

    pub fn try_notify<E>(
        &mut self,
        mut callback: impl FnMut(T, &mut Self) -> Result<(), E>,
    ) -> Result<(), E> {
        self.begin_notification();
        let result = (|| {
            while let Some(value) = self.next_subscriber() {
                callback(value, self)?;
            }
            Ok(())
        })();
        self.end_notification();
        result
    }
}
