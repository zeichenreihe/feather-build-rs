package org.example;

public class FizzBuzz {
	public static void main(String[] args) {
		int n = 100;

		for (int i = 1; i <= 100; i++) {
			boolean mod3 = i % 3 == 0;
			boolean mod5 = i % 5 == 0;
			if (mod3 && mod5) {
				System.out.println("FizzBuzz");
			} else if (mod3) {
				System.out.println("Fizz");
			} else if (mod5) {
				System.out.println("Buzz");
			} else {
				System.out.println(i);
			}
		}
	}
}
