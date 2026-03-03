use rand::Rng;
use std::cmp::Ordering;
use std::io;

fn main() {
    println!("Guess the number!");

    let secret_number = rand::thread_rng().gen_range(1..=100);

    println!("the secret_number is {secret_number}");

    println!("please input your guess.");

    //TODO: 초기화를 한다. let은 뭐지? 변수를 선언하는 키워드.
    //TODO:  mut은 뭐지? rust는 기본적으로 불변. mut는변수의 값을 수정할수 있도록 선언하는 거다.
    //변수가 mutable이 된다.
    let mut guess = String::new();

    // TODO: 1. stdin으로 유저 인풋을 받는다. 2. read_line으로 뭐하는 거지? 3. expect는read_line을
    // 실패했을 때의 동작인가?
    // 2. 유저의 인풋을 받는 함수는 read_line.
    // read_line은 개행문자까지 받는다.
    //
    io::stdin()
        .read_line(&mut guess)
        .expect("you input values from keybord");

    let guess: u32 = guess.trim().parse().expect("Please type a number!");

    println!("check inputted value : {guess}");

    match guess.cmp(&secret_number) {
        Ordering::Less => println!("Too small!"),
        Ordering::Greater => println!("Too big!"),
        Ordering::Equal => println!("You win"),
    };
}
