//! mudd-core — Veterinary ultrasound processing & ML dataset preparation
//!
//! ## Pipeline
//!
//! ```text
//! RawFrame → CroppedFrame → ProcessedFrame → AnnotatedFrame → ExportItem
//! ```
//!
//! Created by M&K (c)2026 VetCoders

pub mod dicom;
pub mod export;
pub mod imaging;
pub mod inference;
pub mod pipeline;
pub mod video;
