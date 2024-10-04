use nalgebra::{DMatrix, DVector};

pub struct AdjacencyMatrix {
    // The matrix stores connectivity, such that 
    // row i gives all outgoing connections for node i
    matrix: Vec<bool>,
    n_nodes: usize
}


impl AdjacencyMatrix {
    pub fn new(size: usize) -> Self {
        AdjacencyMatrix {
            matrix: vec![false; size * size],
            n_nodes: size 
        }
    }
    
    #[inline(always)]
    fn get_row_for_node(&self, node: usize) -> usize {
        node * self.n_nodes
    }
    
    pub fn connect(&mut self, from: usize, to: usize) {
        let row = self.get_row_for_node(from);
        self.matrix[row + to] = true;
    }
    
    pub fn disconnect(&mut self, from: usize, to: usize) {
        let row = self.get_row_for_node(from);
        self.matrix[row + to] =  false;
    }

    #[inline(always)]
    pub fn is_connected(&self, from: usize, to: usize) -> bool {
        self.matrix[self.get_row_for_node(from) + to]
    }
    
    pub fn in_degree(&self, node: usize) -> usize {
        let mut total = 0;
        for index in 0..self.n_nodes {
            total += self.matrix[self.get_row_for_node(index) + node] as usize;
        }
        total 
    }
    
    pub fn out_degree(&self, node: usize) -> usize {
        let mut total = 0;
        let row = self.get_row_for_node(node);
        for index in 0..self.n_nodes {
            total += self.matrix[row + index] as usize;
        }
        total 
    }
    
    // https://www-sciencedirect-com.proxy-ub.rug.nl/science/article/pii/S037843711000991X
    
    pub fn common_neighbour_count(&self, x: usize, y: usize) -> i32 {
        let mut total = 0;
        for index in 0..self.n_nodes {
            if self.matrix[self.get_row_for_node(index) + x] && self.matrix[self.get_row_for_node(index) + y] {
                total += 1;
            }
        }
        total
    }
    
    pub fn salton_metric(&self, x: usize,  y: usize) -> f64 {
        let common = self.common_neighbour_count(x, y) as f64;
        let dx = self.in_degree(x) as f64;
        let dy = self.in_degree(y) as f64;
        common / ((dx * dy).sqrt())
    }
    
    pub fn sorensen_metric(&self, x: usize, y: usize) -> f64 {
        let common = self.common_neighbour_count(x, y) as f64;
        let dx = self.out_degree(x) as f64;
        let dy = self.out_degree(y) as f64;
        common / (dx + dy)
    }
    
    pub fn adamic_adar_metric(&self, x: usize, y: usize) -> f64 {
        let mut total = 0.0;
        for index in 0..self.n_nodes {
            if self.matrix[self.get_row_for_node(index) + x] && self.matrix[self.get_row_for_node(index) + y] {
                total += (self.in_degree(index) as f64).ln()
            }
        }
        total
    }
    
    pub fn russel_rao_metric(&self, x: usize, y: usize) -> f64 {
        let common = self.common_neighbour_count(x, y) as f64;
        common / self.n_nodes as f64
    }
    
    pub fn resource_allocation_metric(&self, x: usize, y: usize) -> f64 {
        let mut total = 0.0;
        for index in 0..self.n_nodes {
            if self.matrix[self.get_row_for_node(index) + x] && self.matrix[self.get_row_for_node(index) + y] {
                let dz = self.in_degree(index) as f64;
                total += 1.0 / dz
            }
        }
        total
    }
    
    pub fn katz_metric(&self) -> GraphMatrix<f64> {
        let A = nalgebra::DMatrix::from_iterator(
            self.n_nodes, self.n_nodes, self.matrix.iter().map(|x| *x as i32 as f64)
        );
        let I = nalgebra::DMatrix::<f64>::identity(self.n_nodes, self.n_nodes);
        let beta = 0.005;
        let B = (&I - beta * A).try_inverse().unwrap();
        let katz = (B - I).data.as_vec().to_vec();
        GraphMatrix { matrix: katz, n_nodes: self.n_nodes }
    }
    
    pub fn sim_rank_metric(&self) -> GraphMatrix<f64> {
        // https://networkx.org/documentation/stable/_modules/networkx/algorithms/similarity.html#simrank_similarity
        let mut A = nalgebra::DMatrix::from_iterator(
            self.n_nodes, self.n_nodes, self.matrix.iter().map(|x| *x as i32 as f64)
        );
        // Column-normalize
        let col_sum = A.column_sum();
        for (col_index, mut s) in col_sum.iter().copied().enumerate() {
            if s == 0.0 {
                s = 1.0;
            };
            let col = DVector::from_element(self.n_nodes, s).component_mul(&A.column(col_index));
            A.set_column(col_index, &col);
        }
        // Iterative procedure 
        let mut iteration = 0;
        let mut current = DMatrix::<f64>::identity(self.n_nodes, self.n_nodes);
        const C: f64 = 0.9;
        loop {
            let previous = current.clone_owned();
            current = C * ((A.transpose() * &previous) * &A);
            current.set_diagonal(&DVector::from_element(self.n_nodes, 1.0));
            
            if (&current - previous).abs().max() < 1e-4 {
                break;
            }
            
            iteration += 1;
            if iteration > 1000 {
                panic!("Did not converge!")
            }
        }
        
        let sim =  current.data.as_vec().to_vec();
        GraphMatrix { matrix: sim, n_nodes: self.n_nodes }
    }
}

pub struct GraphMatrix<T> {
    // The matrix stores connectivity, such that 
    // row i gives all outgoing connections for node i
    pub(super) matrix: Vec<T>,
    pub(super) n_nodes: usize 
}

impl<T: Copy> GraphMatrix<T> {
    pub fn score(&self, x: usize, y: usize) -> T {
        self.matrix[x * self.n_nodes + y]
    }
}
