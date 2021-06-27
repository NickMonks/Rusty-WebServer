use std::thread;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;


pub struct ThreadPool
{
    // Holds a vector of JoinHandle with unit type, since te closure doesnt return
    // any value. we define the senders also, to send and queue up jobs in the channel
    // which will be common
    workers : Vec<Worker>,
    sender : mpsc::Sender<Job>,
}

// define type alias for Job to be a trait object 
// this is the signature of the actual closure sent in pool.execute
type Job = Box<dyn FnOnce() + Send + 'static>;

enum Message {
    NewJob(Job),
    Terminate,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {

        assert!(size > 0);

        // define channel
        let (sender, receiver) = mpsc::channel();
        
        // because the cannot acquire and move a receiver 
        // (only one receiver per channel), we wrap the receiver 
        //into arc (atomic ref counter) to safetly clone and inner mutability, 
        //and a mutex to lock and avoid several threads sending the same job.
        let receiver = Arc::new(Mutex::new(receiver));

        // Allocate some capacity to the vector
        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            // when pushed the new will spawn the threads and run them
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool { workers, sender}
    }

    // FnOnce() takes no parameters and returns unit type
    // We need send to maybe pass the closure to another thread,
    // And static to ensure it lives enough (i.e. the closure might not be executed inmidiately)
    pub fn execute<F>(&self, f: F)
    where
        F : FnOnce() + Send + 'static,
    {
        // This method will send the cloosure to our thread
        let job = Box::new(f);

        self.sender.send(job).unwrap();        
    }
}

impl Drop for ThreadPool {
    // When the threadpool is closed, we want to make sure the threads join
    fn drop(&mut self) {
        for worker in &mut self.workers {
            print!("Shutting down worker {}", worker.id);
            // join takes ownership of the thread, therefore not valid mut ref
            // we wrap the thread in an Option , and take the ownership using take() method
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
            
        }


    }
}


struct Worker {
    thread: Option<thread::JoinHandle<mpsc::Receiver<Job>>>,
    id : usize,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        
        // the threads will be spawned and invoked, but they will be locked until a. acquiring lock; 
        // b. receiving closure/job from the channel
        
        //we want to loop forever, in case one thread acquires lock and finish the job

        let thread = thread::spawn(move || loop {
            // A thread will acquire the lock;   other threads will wait to acquire it
            //If acquired, then it will wait to recieve something from the sender
            
            let job = receiver.lock().unwrap().recv().unwrap();

            println!("Worker {} got a job; executing.", id);

            job();
        });

        Worker { 
            thread: Some(thread), 
            id}
    }
}

