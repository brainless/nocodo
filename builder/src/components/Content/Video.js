const Video = ({
  src,
  className = undefined,
  alt = undefined,
  onClick = undefined,
}) => {
  if (!src) {
    return null;
  }

  return <img src={src} className={className} onClick={onClick} alt={alt} />;
};

export default Video;
