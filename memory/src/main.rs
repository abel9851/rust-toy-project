use std::mem;

const HEAP_SIZE: usize = 1024 * 1024; // 1MB fake heap

// "내가 만든 heap"의 실제 메모리 풀.
// Rust 2024 호환성 룰(static_mut_refs) 때문에, mutable static을 직접 쓰지 않고
// 별도 함수로 raw pointer만 가져오도록 한다.
static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

fn heap_base_ptr() -> *mut u8 {
    core::ptr::addr_of_mut!(HEAP) as *mut u8
}

#[derive(Debug, Clone, Copy)]
struct Block {
    offset: usize, // HEAP 시작으로부터의 오프셋
    size: usize,   // 바이트 단위 크기
    free: bool,
}

#[derive(Debug)]
struct MyHeap {
    blocks: Vec<Block>, // free + used 블록 리스트
}

impl MyHeap {
    fn new() -> Self {
        // 처음에는 HEAP 전체가 하나의 큰 free 블록
        Self {
            blocks: vec![Block {
                offset: 0,
                size: HEAP_SIZE,
                free: true,
            }],
        }
    }

    fn dump_blocks(&self) {
        println!("=== MyHeap blocks ===");
        for (i, b) in self.blocks.iter().enumerate() {
            println!(
                "#{:<2} offset={:<8} size={:<8} free={}",
                i, b.offset, b.size, b.free
            );
        }
        println!("=====================");
    }

    fn alloc(&mut self, size: usize, align: usize) -> Option<*mut u8> {
        // 아주 단순한 first-fit + alignment + block split
        for i in 0..self.blocks.len() {
            if !self.blocks[i].free {
                continue;
            }

            let block = self.blocks[i];

            // alignment 맞추기
            let aligned_offset = align_up(block.offset, align);
            let padding = aligned_offset - block.offset;

            // 이 블록 안에 size만큼 들어갈 수 있는지 확인
            if padding + size > block.size {
                continue;
            }

            // 사용한 뒤 남는 공간
            let remaining = block.size - padding - size;

            // 기존 블록 하나를 [padding free][used][remaining free]로 쪼갠다.
            self.blocks.remove(i);
            let mut insert_at = i;

            if padding > 0 {
                self.blocks.insert(
                    insert_at,
                    Block {
                        offset: block.offset,
                        size: padding,
                        free: true,
                    },
                );
                insert_at += 1;
            }

            self.blocks.insert(
                insert_at,
                Block {
                    offset: aligned_offset,
                    size,
                    free: false,
                },
            );
            insert_at += 1;

            if remaining > 0 {
                self.blocks.insert(
                    insert_at,
                    Block {
                        offset: aligned_offset + size,
                        size: remaining,
                        free: true,
                    },
                );
            }

            let addr = unsafe { heap_base_ptr().add(aligned_offset) as usize };
            let ptr = addr as *mut u8;
            return Some(ptr);
        }

        None
    }

    fn dealloc(&mut self, ptr: *mut u8) {
        // ptr → HEAP 기준 offset으로 되돌리기
        let offset = unsafe { ptr.offset_from(heap_base_ptr()) as usize };

        let idx = self
            .blocks
            .iter()
            .position(|b| b.offset == offset && !b.free)
            .expect("dealloc: invalid pointer (not found in blocks)");

        self.blocks[idx].free = true;

        // 오른쪽 블록과 병합
        if idx + 1 < self.blocks.len() && self.blocks[idx + 1].free {
            let right_size = self.blocks[idx + 1].size;
            self.blocks[idx].size += right_size;
            self.blocks.remove(idx + 1);
        }

        // 왼쪽 블록과 병합
        if idx > 0 && self.blocks[idx - 1].free {
            let size = self.blocks[idx].size;
            self.blocks[idx - 1].size += size;
            self.blocks.remove(idx);
        }
    }
}

fn align_up(x: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    (x + align - 1) & !(align - 1)
}

// "내가 만든 heap" 위에서 동작하는 mini Box<T>
struct MyBox<T> {
    ptr: *mut T,
}

impl<T> MyBox<T> {
    fn new(value: T, heap: &mut MyHeap) -> Option<Self> {
        let size = mem::size_of::<T>();
        let align = mem::align_of::<T>();
        let raw = heap.alloc(size, align)? as *mut T;

        unsafe {
            raw.write(value);
        }

        Some(Self { ptr: raw })
    }

    fn as_ptr(&self) -> *mut T {
        self.ptr
    }

    fn get(&self) -> &T {
        unsafe { &*self.ptr }
    }

    fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.ptr }
    }

    fn free(self, heap: &mut MyHeap) {
        heap.dealloc(self.ptr as *mut u8);
    }
}

fn main() {
    println!("=== process layout 관찰용 ===");

    // 1) "내가 만든 heap"의 실제 메모리 위치
    println!("HEAP (static array) addr      : {:p}", heap_base_ptr());

    // 2) 스택 변수
    let stack_local = 1234_i32;
    println!(
        "stack_local (on stack) addr    : {:p} value={}",
        &stack_local, stack_local
    );

    // 3) 진짜 Rust heap (std::Box)와 비교
    let real_box = Box::new(7777_i32);
    println!(
        "real_box (Box, on stack) addr  : {:p}",
        &real_box
    );
    println!(
        "*real_box (on real heap) addr  : {:p} value={}",
        &*real_box, *real_box
    );

    // 4) 나만의 heap 초기화
    let mut my_heap = MyHeap::new();
    my_heap.dump_blocks();

    // 5) MyBox로 "내 heap" 위에 값 올리기
    let mut my_a = MyBox::new(10_i32, &mut my_heap).expect("alloc failed");
    let mut my_b = MyBox::new(20_i32, &mut my_heap).expect("alloc failed");
    let mut my_c = MyBox::new(30_i32, &mut my_heap).expect("alloc failed");

    println!(
        "my_a (MyBox struct, stack) addr: {:p}, value={}, inner ptr={:p}",
        &my_a,
        my_a.get(),
        my_a.as_ptr()
    );
    println!(
        "my_b (MyBox struct, stack) addr: {:p}, value={}, inner ptr={:p}",
        &my_b,
        my_b.get(),
        my_b.as_ptr()
    );
    println!(
        "my_c (MyBox struct, stack) addr: {:p}, value={}, inner ptr={:p}",
        &my_c,
        my_c.get(),
        my_c.as_ptr()
    );

    println!();
    println!(
        "size_of::<MyBox<i32>>() = {} bytes (stack에 놓이는 핸들 크기)",
        mem::size_of::<MyBox<i32>>()
    );
    println!(
        "size_of::<i32>()        = {} bytes (내 heap 안에 실제 데이터 크기)",
        mem::size_of::<i32>()
    );

    my_heap.dump_blocks();

    // 6) 내 heap 위 데이터 수정해보기
    *my_a.get_mut() = 111;
    *my_b.get_mut() = 222;
    *my_c.get_mut() = 333;

    println!(
        "after mutation: my_a={}, my_b={}, my_c={}",
        my_a.get(),
        my_b.get(),
        my_c.get()
    );

    // 7) 일부 free 해서 병합(coalesce) 확인
    println!("\n-- free my_b --");
    my_b.free(&mut my_heap);
    my_heap.dump_blocks();

    println!("\n-- free my_a --");
    my_a.free(&mut my_heap);
    my_heap.dump_blocks();

    println!("\n-- free my_c --");
    my_c.free(&mut my_heap);
    my_heap.dump_blocks();

    println!("done.");
}
