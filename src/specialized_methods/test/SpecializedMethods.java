// Example from https://docs.oracle.com/javase/tutorial/java/generics/bridgeMethods.html

class SpecializedMethods {
    public static void main(String[] args) {
        MyNode mn = new MyNode(5);
        Node n = mn;            // A raw type - compiler throws an unchecked warning
        n.setData("Hello");     // Causes a ClassCastException to be thrown.
        Integer x = mn.data;
    }
}

/*public*/ class Node<T> {

    public T data;

    public Node(T data) { this.data = data; }

    public void setData(T data) {
        System.out.println("Node.setData");
        this.data = data;
    }
}

/*public*/ class MyNode extends Node<Integer> {
    public MyNode(Integer data) { super(data); }

    public void setData(Integer data) {
        System.out.println("MyNode.setData");
        super.setData(data);
    }

    // Bridge method generated by the compiler
    //
    //public void setData(Object data) {
    //    setData((Integer) data);
    //}
}