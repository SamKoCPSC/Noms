export default function formatTimestamp(timestamp) {
  let isoTimestamp;
  
  if (typeof timestamp === 'string') {
    // If it's already a string, convert PostgreSQL format to ISO format
    isoTimestamp = timestamp.replace(" ", "T");
  } else if (timestamp instanceof Date) {
    // If it's already a Date object, use it directly
    return timestamp;
  } else if (timestamp && typeof timestamp.toISOString === 'function') {
    // If it's a date-like object with toISOString, use that
    return timestamp;
  } else {
    // Handle null or undefined values
    return 'Invalid date';
  }
  
  const date = new Date(isoTimestamp);
  if (isNaN(date.getTime())) {
    throw new Error("Invalid PostgreSQL timestamp format.");
  }
  
  return date;
}