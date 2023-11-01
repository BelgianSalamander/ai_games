use async_trait::async_trait;

pub struct Pool<T> {
    items: Vec<T>,

    next_free: usize,
    tail: usize,
    links: Vec<usize>,

    num_available: usize,
}

const INVALID_INDEX: usize = usize::MAX;
const IN_USE_INDEX: usize = usize::MAX - 1;

impl<T> Pool<T> {
    pub fn new<F: Fn(usize) -> T>(num_items: usize, supplier: F) -> Self {
        let mut items = Vec::with_capacity(num_items);
        let mut links = Vec::with_capacity(num_items);

        for i in 0..num_items {
            if i == num_items - 1 {
                links.push(INVALID_INDEX);
            } else {
                links.push(i + 1);
            }

            items.push(supplier(i));
        }

        Self {
            items,
            next_free: 0,
            tail: num_items - 1,
            links,

            num_available: num_items,
        }
    }

    pub async fn new_async<Closure, Future>(num_items: usize, supplier: Closure) -> Self where 
        Closure: Fn(usize) -> Future,
        Future: std::future::Future<Output = T> + Send + 'static,
    {
        println!("Creating pool of {} items asynchronously", num_items);
        let mut item_futures = Vec::with_capacity(num_items);
        let mut links = Vec::with_capacity(num_items);

        for i in 0..num_items {
            if i == num_items - 1 {
                links.push(INVALID_INDEX);
            } else {
                links.push(i + 1);
            }

            item_futures.push(supplier(i));
        }

        let items = futures::future::join_all(item_futures).await;

        Self {
            items,
            next_free: 0,
            tail: num_items - 1,
            links,

            num_available: num_items,
        }
    }

    pub fn num_available(&self) -> usize {
        self.num_available
    }

    pub fn get(&mut self) -> Option<(usize, &mut T)> {
        if self.num_available == 0 {
            return None;
        }

        let index = self.next_free;
        self.next_free = self.links[index];
        self.links[index] = IN_USE_INDEX;

        self.num_available -= 1;

        if self.tail == index {
            self.tail = INVALID_INDEX;
        }

        Some((index, &mut self.items[index]))
    }

    pub fn release(&mut self, index: usize) {
        if self.links[index] == IN_USE_INDEX {
            self.links[index] = self.next_free;
            self.next_free = index;

            if self.tail == INVALID_INDEX {
                self.tail = index;
            }

            self.num_available += 1;
        }
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if self.links[index] == IN_USE_INDEX {
            Some(&mut self.items[index])
        } else {
            None
        }
    }
}